#!/usr/bin/env bash
# Downloads the IP geolocation databases by hand: the city database (country, region,
# city, coordinates) and the ASN database (network owner). Lookups are then local; no
# sign-in ever calls an outside service.
#
# Usually not needed: with GEOIP_DIR set, the server downloads DB-IP Lite itself and
# keeps it current (infraestructure/geoip/scheduler.rs). Use this script for MaxMind
# GeoLite2, which needs an account, or for a host where the server has no outbound
# access. The server never replaces a GeoLite2 file this script installed, so keep
# running the script on a schedule in that case.
#
#   scripts/geoip-update.sh                       # into server/.geoip
#   GEOIP_DIR=/var/lib/geoip scripts/geoip-update.sh
#
# Source, picked automatically:
#   - MaxMind GeoLite2, when MAXMIND_ACCOUNT_ID and MAXMIND_LICENSE_KEY are set (free
#     account at maxmind.com). Updated twice a week; the most accurate free city data.
#     The download is checked against MaxMind's published SHA-256.
#   - DB-IP Lite otherwise: no account. Updated monthly, licensed CC BY 4.0, so the
#     console map credits it (the server reads the database type from the file).
#
# Output: $GEOIP_DIR/city.mmdb and $GEOIP_DIR/asn.mmdb, each replaced in one rename
# once complete and checked, so a failed run leaves the old files in place. A running
# server picks up a file this script installs only at its next restart. Both licences
# expect the data kept current (docs/fraud-specification.md §12.4).
set -euo pipefail

cd "$(dirname "$0")/.."
DIR="${GEOIP_DIR:-.geoip}"
mkdir -p "$DIR"
TMP="$(mktemp -d "$DIR/.download.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

fetch() {
	curl --fail --silent --show-error --location --retry 3 --max-time 900 \
		-A "gatekeeper-geoip-update" "$@"
}

# An MMDB file ends with its metadata section, which starts with this marker.
is_mmdb() {
	tail -c 131072 "$1" | grep -qa 'MaxMind.com'
}

# DB-IP publishes one file per month; early in a month the new one may not be out yet.
dbip() {
	local kind=$1 out=$2 month url
	for month in "$(date -u +%Y-%m)" "$(date -u -d "$(date -u +%Y-%m-01) -1 month" +%Y-%m)"; do
		url="https://download.db-ip.com/free/dbip-$kind-lite-$month.mmdb.gz"
		if fetch -o "$TMP/$kind.mmdb.gz" "$url" 2>/dev/null; then
			gzip -t "$TMP/$kind.mmdb.gz"
			gzip -dc "$TMP/$kind.mmdb.gz" >"$TMP/$out"
			echo "DB-IP $kind lite $month"
			return 0
		fi
	done
	echo "could not download DB-IP $kind lite for this month or the last" >&2
	return 1
}

# The key goes in a netrc file, not on the command line (where `ps` shows it), and
# curl sends it only to download.maxmind.com, not to the storage host it redirects to.
maxmind() {
	local edition=$1 out=$2
	local base="https://download.maxmind.com/geoip/databases/$edition/download"
	local expected actual found
	fetch --netrc-file "$TMP/netrc" -o "$TMP/$edition.tar.gz" "$base?suffix=tar.gz"
	fetch --netrc-file "$TMP/netrc" -o "$TMP/$edition.sha256" "$base?suffix=tar.gz.sha256"
	expected="$(awk '{print $1}' "$TMP/$edition.sha256")"
	actual="$(sha256sum "$TMP/$edition.tar.gz" | awk '{print $1}')"
	if [[ -z "$expected" || "$expected" != "$actual" ]]; then
		echo "$edition: checksum mismatch (expected '$expected', got '$actual')" >&2
		return 1
	fi
	mkdir -p "$TMP/$edition"
	tar -xzf "$TMP/$edition.tar.gz" -C "$TMP/$edition"
	found="$(find "$TMP/$edition" -type f -name "$edition.mmdb" | head -n 1)"
	if [[ -z "$found" ]]; then
		echo "$edition: no $edition.mmdb in the archive" >&2
		return 1
	fi
	mv "$found" "$TMP/$out"
	echo "MaxMind $edition $(basename "$(dirname "$found")" | sed "s/^${edition}_//")"
}

if [[ -n "${MAXMIND_ACCOUNT_ID:-}" && -n "${MAXMIND_LICENSE_KEY:-}" ]]; then
	(
		umask 077
		printf 'machine download.maxmind.com login %s password %s\n' \
			"$MAXMIND_ACCOUNT_ID" "$MAXMIND_LICENSE_KEY" >"$TMP/netrc"
	)
	maxmind GeoLite2-City city.mmdb
	maxmind GeoLite2-ASN asn.mmdb
else
	dbip city city.mmdb
	dbip asn asn.mmdb
fi

for file in city.mmdb asn.mmdb; do
	if ! is_mmdb "$TMP/$file"; then
		echo "$file: not an MMDB database; keeping the current one" >&2
		exit 1
	fi
done
mv -f "$TMP/city.mmdb" "$DIR/city.mmdb"
mv -f "$TMP/asn.mmdb" "$DIR/asn.mmdb"

abs="$(cd "$DIR" && pwd)"
cat <<EOF
Installed:
  $abs/city.mmdb ($(du -h "$abs/city.mmdb" | cut -f1))
  $abs/asn.mmdb ($(du -h "$abs/asn.mmdb" | cut -f1))

Point the server at them (then restart it):
  GEOIP_CITY_DB_PATH=$abs/city.mmdb
  GEOIP_ASN_DB_PATH=$abs/asn.mmdb
EOF
