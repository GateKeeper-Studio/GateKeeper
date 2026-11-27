// Keeps the DB-IP Lite databases in GEOIP_DIR current, so a server started with only
// GEOIP_DIR set downloads its geolocation data by itself and refreshes it when DB-IP
// publishes a new month. DB-IP Lite needs no account: one public file per month and
// edition, `https://download.db-ip.com/free/dbip-{city,asn}-lite-YYYY-MM.mmdb.gz`,
// published on the 1st (a 404 before then).
//
// The server never replaces a file it did not download: a GeoLite2 file put there by
// `scripts/geoip-update.sh` (MaxMind, with an account) is left alone. A new file is
// written next to the old one, opened and checked like the server will use it, and
// only then renamed over it, so a failed or partial download changes nothing.
//
// Blocking work (decompressing, reading the file) runs here as plain functions; the
// scheduler calls them on the blocking pool.

use std::{
    fmt,
    io::Read,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Datelike, Utc};
use flate2::read::GzDecoder;
use maxminddb::{Reader, geoip2};

// The compressed downloads are about 60 MB (city) and 5 MB (ASN); decompressed,
// 130 MB and 10 MB. Generous caps, so a growing file still fits but a hostile or
// broken response cannot fill the disk or the memory.
pub const MAX_COMPRESSED_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_DECOMPRESSED_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    City,
    Asn,
}

impl Edition {
    fn slug(self) -> &'static str {
        match self {
            Edition::City => "city",
            Edition::Asn => "asn",
        }
    }

    // The `database_type` in the file's metadata: `DBIP-City-Lite`,
    // `DBIP-ASN-Lite (compat=GeoLite2-ASN)`.
    fn matches(self, database_type: &str) -> bool {
        match self {
            Edition::City => database_type.starts_with("DBIP-City-Lite"),
            Edition::Asn => database_type.starts_with("DBIP-ASN-Lite"),
        }
    }
}

impl fmt::Display for Edition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

// A DB-IP release: one calendar month (UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Month {
    year: i32,
    month: u32,
}

impl Month {
    pub fn of(at: DateTime<Utc>) -> Self {
        Month {
            year: at.year(),
            month: at.month(),
        }
    }

    fn from_epoch(seconds: u64) -> Option<Self> {
        DateTime::from_timestamp(i64::try_from(seconds).ok()?, 0).map(Month::of)
    }

    pub fn previous(self) -> Self {
        if self.month == 1 {
            Month {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Month {
                year: self.year,
                month: self.month - 1,
            }
        }
    }
}

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

pub fn download_url(edition: Edition, month: Month) -> String {
    format!(
        "https://download.db-ip.com/free/dbip-{}-lite-{month}.mmdb.gz",
        edition.slug()
    )
}

// What is on disk where an edition's file belongs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Local {
    Missing,
    // Present but not a readable MMDB file (e.g. a download cut short by a crash
    // before this updater existed): replaced.
    Unreadable,
    DbIpLite(Month),
    // Some other database (GeoLite2, a paid one): never replaced.
    Foreign(String),
}

pub fn inspect(path: &Path, edition: Edition) -> Local {
    if !path.exists() {
        return Local::Missing;
    }
    let Ok(reader) = Reader::open_readfile(path) else {
        return Local::Unreadable;
    };
    let metadata = reader.metadata();
    if !edition.matches(&metadata.database_type) {
        return Local::Foreign(metadata.database_type.clone());
    }
    match Month::from_epoch(metadata.build_epoch) {
        Some(month) => Local::DbIpLite(month),
        None => Local::Unreadable,
    }
}

// The releases worth trying, newest first. Nothing when the file is current or not
// ours. A missing file may fall back to last month's, since this month's appears only
// on the 1st; an outdated one is replaced only by something newer.
pub fn months_to_try(local: &Local, now: DateTime<Utc>) -> Vec<Month> {
    let current = Month::of(now);
    let previous = current.previous();
    match local {
        Local::Foreign(_) => Vec::new(),
        Local::DbIpLite(have) if *have >= current => Vec::new(),
        Local::DbIpLite(have) if *have >= previous => vec![current],
        Local::DbIpLite(_) | Local::Missing | Local::Unreadable => vec![current, previous],
    }
}

#[derive(Debug)]
pub enum InstallError {
    Io(std::io::Error),
    TooLarge,
    NotADatabase(String),
    WrongEdition { expected: Edition, found: String },
    Implausible(&'static str),
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallError::Io(e) => write!(f, "i/o error: {e}"),
            InstallError::TooLarge => write!(f, "decompressed file exceeds the size cap"),
            InstallError::NotADatabase(e) => write!(f, "not an MMDB database: {e}"),
            InstallError::WrongEdition { expected, found } => {
                write!(
                    f,
                    "expected the DB-IP {expected} Lite database, got '{found}'"
                )
            }
            InstallError::Implausible(why) => write!(f, "rejected: {why}"),
        }
    }
}

impl From<std::io::Error> for InstallError {
    fn from(e: std::io::Error) -> Self {
        InstallError::Io(e)
    }
}

// Decompresses `gzip` next to `dest`, checks it is the expected DB-IP database and
// answers a known public address, then renames it over `dest`. Returns its release.
pub fn install(gzip: &[u8], edition: Edition, dest: &Path) -> Result<Month, InstallError> {
    install_capped(gzip, edition, dest, MAX_DECOMPRESSED_BYTES)
}

fn install_capped(
    gzip: &[u8],
    edition: Edition,
    dest: &Path,
    max_bytes: u64,
) -> Result<Month, InstallError> {
    let dir = dest.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let partial = partial_path(dest);

    let result = write_decompressed(gzip, &partial, max_bytes).and_then(|()| {
        let release = verify(&partial, edition)?;
        std::fs::rename(&partial, dest)?;
        Ok(release)
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result
}

// `.city.mmdb.partial-<uuid>`: hidden, unique per attempt, in the same directory so
// the final rename is atomic.
fn partial_path(dest: &Path) -> PathBuf {
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "geoip.mmdb".to_string());
    dest.with_file_name(format!(".{name}.partial-{}", uuid::Uuid::now_v7()))
}

fn write_decompressed(gzip: &[u8], to: &Path, max_bytes: u64) -> Result<(), InstallError> {
    let mut out = std::fs::File::create(to)?;
    let mut decoder = GzDecoder::new(gzip).take(max_bytes + 1);
    let written = std::io::copy(&mut decoder, &mut out)?;
    if written > max_bytes {
        return Err(InstallError::TooLarge);
    }
    out.sync_all()?;
    Ok(())
}

fn verify(path: &Path, edition: Edition) -> Result<Month, InstallError> {
    let reader =
        Reader::open_readfile(path).map_err(|e| InstallError::NotADatabase(e.to_string()))?;
    let metadata = reader.metadata();
    if !edition.matches(&metadata.database_type) {
        return Err(InstallError::WrongEdition {
            expected: edition,
            found: metadata.database_type.clone(),
        });
    }
    let release = Month::from_epoch(metadata.build_epoch)
        .ok_or(InstallError::Implausible("no build date"))?;

    // 8.8.8.8 is in every real edition; an empty or truncated file would not place it.
    let probe = "8.8.8.8".parse().expect("valid address");
    let found = reader
        .lookup(probe)
        .map_err(|e| InstallError::NotADatabase(e.to_string()))?;
    let plausible = match edition {
        Edition::City => found
            .decode::<geoip2::City>()
            .ok()
            .flatten()
            .and_then(|c| c.country.iso_code.map(str::to_string))
            .is_some(),
        Edition::Asn => found
            .decode::<geoip2::Asn>()
            .ok()
            .flatten()
            .and_then(|a| a.autonomous_system_number)
            .is_some(),
    };
    if !plausible {
        return Err(InstallError::Implausible("does not place 8.8.8.8"));
    }
    Ok(release)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use chrono::TimeZone;
    use flate2::{Compression, write::GzEncoder};

    use super::*;

    fn at(year: i32, month: u32, day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap()
    }

    fn month(year: i32, month: u32) -> Month {
        Month { year, month }
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }

    fn scratch_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gk-geoip-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn urls_name_the_edition_and_the_month() {
        assert_eq!(
            download_url(Edition::City, month(2026, 3)),
            "https://download.db-ip.com/free/dbip-city-lite-2026-03.mmdb.gz"
        );
        assert_eq!(
            download_url(Edition::Asn, month(2026, 11)),
            "https://download.db-ip.com/free/dbip-asn-lite-2026-11.mmdb.gz"
        );
    }

    #[test]
    fn january_comes_after_last_december() {
        assert_eq!(month(2027, 1).previous(), month(2026, 12));
        assert_eq!(month(2026, 7).previous(), month(2026, 6));
    }

    #[test]
    fn a_missing_file_falls_back_to_last_month() {
        assert_eq!(
            months_to_try(&Local::Missing, at(2026, 12, 1)),
            vec![month(2026, 12), month(2026, 11)]
        );
        assert_eq!(
            months_to_try(&Local::Unreadable, at(2027, 1, 1)),
            vec![month(2027, 1), month(2026, 12)]
        );
    }

    #[test]
    fn a_current_file_is_left_alone() {
        assert!(months_to_try(&Local::DbIpLite(month(2026, 11)), at(2026, 11, 26)).is_empty());
    }

    #[test]
    fn last_months_file_waits_for_this_months_only() {
        assert_eq!(
            months_to_try(&Local::DbIpLite(month(2026, 11)), at(2026, 12, 3)),
            vec![month(2026, 12)]
        );
    }

    #[test]
    fn an_older_file_takes_either_recent_month() {
        assert_eq!(
            months_to_try(&Local::DbIpLite(month(2026, 6)), at(2026, 12, 1)),
            vec![month(2026, 12), month(2026, 11)]
        );
    }

    #[test]
    fn someone_elses_database_is_never_replaced() {
        let local = Local::Foreign("GeoLite2-City".to_string());
        assert!(months_to_try(&local, at(2026, 12, 3)).is_empty());
    }

    #[test]
    fn the_editions_recognise_their_own_files() {
        assert!(Edition::City.matches("DBIP-City-Lite"));
        assert!(Edition::Asn.matches("DBIP-ASN-Lite (compat=GeoLite2-ASN)"));
        assert!(!Edition::City.matches("DBIP-ASN-Lite (compat=GeoLite2-ASN)"));
        assert!(!Edition::City.matches("GeoLite2-City"));
        assert!(!Edition::City.matches("DBIP-Location-ISP (compat=Enterprise)"));
    }

    #[test]
    fn a_missing_file_inspects_as_missing_and_junk_as_unreadable() {
        let dir = scratch_dir();
        assert_eq!(
            inspect(&dir.join("city.mmdb"), Edition::City),
            Local::Missing
        );
        std::fs::write(dir.join("city.mmdb"), b"not a database").unwrap();
        assert_eq!(
            inspect(&dir.join("city.mmdb"), Edition::City),
            Local::Unreadable
        );
    }

    #[test]
    fn a_bad_download_leaves_the_current_file_and_no_partial() {
        let dir = scratch_dir();
        let dest = dir.join("city.mmdb");
        std::fs::write(&dest, b"the current file").unwrap();

        // Not gzip at all.
        assert!(matches!(
            install(b"<html>error page</html>", Edition::City, &dest),
            Err(InstallError::Io(_))
        ));
        // Valid gzip, but not a database.
        assert!(matches!(
            install(&gzip(b"hello"), Edition::City, &dest),
            Err(InstallError::NotADatabase(_))
        ));
        // Bigger than the cap.
        assert!(matches!(
            install_capped(&gzip(&[0u8; 4096]), Edition::City, &dest, 1024),
            Err(InstallError::TooLarge)
        ));

        assert_eq!(std::fs::read(&dest).unwrap(), b"the current file");
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .filter(|n| n.to_string_lossy().contains(".partial-"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    // Round-trips a real DB-IP file through `install`, when one is configured:
    // GEOIP_DIR=… cargo test --lib dbip -- --ignored
    #[test]
    #[ignore = "needs a downloaded DB-IP database in GEOIP_DIR"]
    fn a_real_database_installs_and_reports_its_month() {
        let source = PathBuf::from(std::env::var("GEOIP_DIR").expect("GEOIP_DIR"));
        let dir = scratch_dir();
        for (edition, file) in [(Edition::City, "city.mmdb"), (Edition::Asn, "asn.mmdb")] {
            let original = std::fs::read(source.join(file)).unwrap();
            let Local::DbIpLite(expected) = inspect(&source.join(file), edition) else {
                panic!("{file} is not a DB-IP Lite database");
            };
            let release = install(&gzip(&original), edition, &dir.join(file)).unwrap();
            assert_eq!(release, expected);
            assert_eq!(inspect(&dir.join(file), edition), Local::DbIpLite(expected));
            // The wrong edition is refused.
            let other = if edition == Edition::City {
                Edition::Asn
            } else {
                Edition::City
            };
            assert!(matches!(
                install(&gzip(&original), other, &dir.join("other.mmdb")),
                Err(InstallError::WrongEdition { .. })
            ));
        }
    }
}
