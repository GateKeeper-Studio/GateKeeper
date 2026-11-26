#!/usr/bin/env bash
# Local HTTPS for development: a private certificate authority and a certificate for
# this machine's LAN address, so a phone on the same Wi-Fi gets a secure context
# (WebAuthn / passkeys, crypto.subtle, clipboard) from the dev server.
#
#   scripts/dev-tls.sh            # detect the LAN IP
#   scripts/dev-tls.sh 10.0.0.12  # or name it
#
# The certificate is for <ip-with-dashes>.sslip.io: a public DNS name that resolves
# to the IP inside it (192-168-0-208.sslip.io -> 192.168.0.208). WebAuthn needs a
# real host name; an IP address cannot be a relying-party id. The IP itself is in the
# certificate too.
#
# The CA is created once and reused; only the server certificate is reissued (e.g.
# when the IP changes), so the phone keeps trusting it. Its name constraints limit it
# to sslip.io / nip.io / localhost names and private IP ranges: if ca.key ever
# leaked, it could not impersonate any other site.
#
# Output (git-ignored): server/.dev-tls/{ca.key, gatekeeper-dev-ca.crt, server.key, server.crt}
set -euo pipefail

cd "$(dirname "$0")/.."
OUT=.dev-tls
IP="${1:-$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{for (i = 1; i < NF; i++) if ($i == "src") print $(i + 1)}')}"
if [[ ! "$IP" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
	echo "could not detect the LAN IP; pass it: $0 <ip>" >&2
	exit 1
fi
HOST="${IP//./-}.sslip.io"

mkdir -p "$OUT"
chmod 700 "$OUT"
umask 077

if [[ ! -f "$OUT/ca.key" || ! -f "$OUT/gatekeeper-dev-ca.crt" ]]; then
	openssl ecparam -name prime256v1 -genkey -noout | openssl pkcs8 -topk8 -nocrypt -out "$OUT/ca.key"
	cat > "$OUT/ca.cnf" <<CNF
[req]
distinguished_name = dn
prompt = no
x509_extensions = ca
[dn]
CN = GateKeeper local development CA ($(hostname))
O = GateKeeper development
[ca]
basicConstraints = critical, CA:TRUE, pathlen:0
keyUsage = critical, keyCertSign, cRLSign
subjectKeyIdentifier = hash
nameConstraints = critical, @constraints
[constraints]
permitted;DNS.1 = sslip.io
permitted;DNS.2 = nip.io
permitted;DNS.3 = localhost
permitted;IP.1 = 10.0.0.0/255.0.0.0
permitted;IP.2 = 172.16.0.0/255.240.0.0
permitted;IP.3 = 192.168.0.0/255.255.0.0
permitted;IP.4 = 127.0.0.0/255.0.0.0
CNF
	openssl req -x509 -new -key "$OUT/ca.key" -sha256 -days 1095 -config "$OUT/ca.cnf" \
		-out "$OUT/gatekeeper-dev-ca.crt"
	rm -f "$OUT/ca.cnf"
	echo "created the CA: $OUT/gatekeeper-dev-ca.crt (install it on the phone once)"
fi

openssl ecparam -name prime256v1 -genkey -noout | openssl pkcs8 -topk8 -nocrypt -out "$OUT/server.key"
openssl req -new -key "$OUT/server.key" -subj "/CN=$HOST" -out "$OUT/server.csr"
cat > "$OUT/server.ext" <<EXT
basicConstraints = critical, CA:FALSE
keyUsage = critical, digitalSignature
extendedKeyUsage = serverAuth
subjectAltName = DNS:$HOST, IP:$IP
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid
EXT
# 397 days: the longest validity browsers accept for a server certificate.
openssl x509 -req -in "$OUT/server.csr" -CA "$OUT/gatekeeper-dev-ca.crt" -CAkey "$OUT/ca.key" \
	-set_serial "0x$(openssl rand -hex 16)" -days 397 -sha256 -extfile "$OUT/server.ext" \
	-out "$OUT/server.crt" 2>/dev/null
rm -f "$OUT/server.csr" "$OUT/server.ext"
chmod 644 "$OUT/gatekeeper-dev-ca.crt" "$OUT/server.crt"

cat <<MSG
issued $OUT/server.crt for https://$HOST (and $IP)

server/.env:
  TLS_CERT_FILE=".dev-tls/server.crt"
  TLS_KEY_FILE=".dev-tls/server.key"
  TLS_PORT="8443"
  AUTH_UI_BASE_URL="https://$HOST:8443"
  WEBAUTHN_RP_ORIGIN="https://$HOST:8443"
  WEBAUTHN_RP_ID="$HOST"

Then trust $OUT/gatekeeper-dev-ca.crt on each device that opens the site (once):
  Android: Settings > Security & privacy > More security settings > Encryption &
           credentials > Install a certificate > CA certificate
  Chrome on this Linux desktop:
           certutil -d sql:\$HOME/.pki/nssdb -A -t "C,," -n "GateKeeper dev CA" -i $OUT/gatekeeper-dev-ca.crt
MSG
