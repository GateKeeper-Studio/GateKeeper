use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration as StdDuration,
};

use chrono::Utc;
use reqwest::StatusCode;
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{self, Duration, MissedTickBehavior},
};

use crate::infraestructure::{
    config::GeoIpConfig,
    geoip::{
        dbip_updater::{
            self, Edition, Local, MAX_COMPRESSED_BYTES, download_url, inspect, months_to_try,
        },
        maxmind_geoip_resolver::MaxmindGeoIpResolver,
        reloadable_geoip_resolver::ReloadableGeoIpResolver,
    },
};

/// Background GeoIP refresh (GEOIP_DIR with GEOIP_AUTO_UPDATE, the default).
///
/// WHY: geolocation was only as good as whoever remembered to download a database and
/// keep it current. With this, starting the server is enough. The first tick (at
/// startup) downloads whatever is missing. Later ticks fetch DB-IP's new month once it
/// is out, and each install is swapped into the running resolver, so no restart is
/// needed.
///
/// Off the request path entirely: sign-ins read whatever database is loaded (none yet
/// on a first start, which only means no geo signals until the download finishes).
/// Safe on several replicas sharing a directory: each one downloads, checks and
/// renames atomically, so the last rename wins and every copy is complete.
pub fn spawn_geoip_updater(
    config: &GeoIpConfig,
    resolver: Arc<ReloadableGeoIpResolver>,
) -> (JoinHandle<()>, watch::Sender<bool>) {
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let city = PathBuf::from(&config.city_db_path);
    let asn = config.asn_db_path.as_ref().map(PathBuf::from);
    let interval_hours = config.update_interval_hours;

    let handle = tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .user_agent("gatekeeper-geoip-update")
            .connect_timeout(StdDuration::from_secs(30))
            .timeout(StdDuration::from_secs(15 * 60))
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                tracing::error!("GeoIP updater not started: {e}");
                return;
            }
        };

        let mut ticker = time::interval(Duration::from_secs(interval_hours * 3600));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        tracing::info!(interval_hours, dir = %city.parent().map(|d| d.display().to_string()).unwrap_or_default(), "GeoIP updater started");

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    refresh(&client, &city, asn.as_deref(), &resolver).await;
                }
                _ = shutdown_rx.changed() => break,
            }
        }
        tracing::info!("GeoIP updater stopped");
    });

    (handle, shutdown_tx)
}

// One pass: bring each edition up to date, then reload if anything changed.
async fn refresh(
    client: &reqwest::Client,
    city: &Path,
    asn: Option<&Path>,
    resolver: &ReloadableGeoIpResolver,
) {
    let mut editions = vec![(Edition::City, city.to_path_buf())];
    if let Some(asn) = asn {
        editions.push((Edition::Asn, asn.to_path_buf()));
    }

    let mut installed = false;
    for (edition, path) in editions {
        installed |= update_edition(client, edition, path).await;
    }
    if !installed {
        return;
    }

    let city = city.to_string_lossy().into_owned();
    let asn = asn.map(|p| p.to_string_lossy().into_owned());
    let reopened =
        tokio::task::spawn_blocking(move || MaxmindGeoIpResolver::open(&city, asn.as_deref()))
            .await;
    match reopened {
        Ok(Some(next)) => {
            tracing::info!(
                database = next.database_type(),
                "GeoIP database loaded; sign-ins are geolocated with it from now on"
            );
            resolver.replace(Arc::new(next));
        }
        // `open` already logged why.
        Ok(None) => {}
        Err(e) => tracing::error!("GeoIP reload panicked: {e}"),
    }
}

// Returns true when a new file was installed.
async fn update_edition(client: &reqwest::Client, edition: Edition, path: PathBuf) -> bool {
    let probe = path.clone();
    let local = match tokio::task::spawn_blocking(move || inspect(&probe, edition)).await {
        Ok(local) => local,
        Err(e) => {
            tracing::error!("GeoIP {edition}: inspecting the current file panicked: {e}");
            return false;
        }
    };
    if let Local::Foreign(database_type) = &local {
        tracing::debug!(
            edition = %edition,
            database_type,
            "GeoIP file is not a DB-IP Lite download; left for its owner to update"
        );
        return false;
    }

    for month in months_to_try(&local, Utc::now()) {
        let url = download_url(edition, month);
        let gzip = match download(client, &url).await {
            Ok(Some(bytes)) => bytes,
            // Not published yet (early on the 1st): try the previous month, or wait for
            // the next tick.
            Ok(None) => continue,
            Err(e) => {
                tracing::warn!(edition = %edition, %month, "GeoIP download failed: {e}");
                return false;
            }
        };

        let dest = path.clone();
        match tokio::task::spawn_blocking(move || dbip_updater::install(&gzip, edition, &dest))
            .await
        {
            Ok(Ok(release)) => {
                tracing::info!(
                    edition = %edition,
                    release = %release,
                    path = %path.display(),
                    "GeoIP database installed (DB-IP Lite)"
                );
                return true;
            }
            Ok(Err(e)) => {
                tracing::warn!(edition = %edition, %month, "GeoIP download rejected: {e}");
                return false;
            }
            Err(e) => {
                tracing::error!(edition = %edition, "GeoIP install panicked: {e}");
                return false;
            }
        }
    }
    false
}

// The whole body, or None for a 404 (that month is not out yet). Bounded by
// MAX_COMPRESSED_BYTES whatever the server claims.
async fn download(client: &reqwest::Client, url: &str) -> Result<Option<Vec<u8>>, String> {
    let mut response = client.get(url).send().await.map_err(|e| e.to_string())?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    if response
        .content_length()
        .is_some_and(|len| len > MAX_COMPRESSED_BYTES)
    {
        return Err("larger than the download cap".to_string());
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if (body.len() + chunk.len()) as u64 > MAX_COMPRESSED_BYTES {
            return Err("larger than the download cap".to_string());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Some(body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::ports::geoip_resolver::GeoIpResolver,
        infraestructure::geoip::maxmind_geoip_resolver::NoopGeoIpResolver,
    };

    // The first start, for real: an empty directory, DB-IP over the network, then the
    // running resolver answering with the new database. About 65 MB.
    //   cargo test --lib geoip::scheduler -- --ignored
    #[tokio::test]
    #[ignore = "downloads the DB-IP Lite databases"]
    async fn a_first_start_downloads_both_databases_and_swaps_them_in() {
        let dir =
            std::env::temp_dir().join(format!("gk-geoip-first-start-{}", uuid::Uuid::now_v7()));
        let city = dir.join("city.mmdb");
        let asn = dir.join("asn.mmdb");
        let resolver = ReloadableGeoIpResolver::new(Arc::new(NoopGeoIpResolver));
        assert_eq!(resolver.lookup("8.8.8.8"), None);

        let client = reqwest::Client::builder()
            .user_agent("gatekeeper-geoip-update")
            .build()
            .unwrap();
        refresh(&client, &city, Some(asn.as_path()), &resolver).await;

        let google = resolver
            .lookup("8.8.8.8")
            .expect("located after the download");
        assert_eq!(google.country_code.as_deref(), Some("US"));
        assert_eq!(google.asn, Some(15169));
        assert_eq!(
            resolver.attribution().map(|a| a.url),
            Some("https://db-ip.com")
        );

        // A second pass finds both current and downloads nothing.
        let before = std::fs::metadata(&city).unwrap().modified().unwrap();
        refresh(&client, &city, Some(asn.as_path()), &resolver).await;
        assert_eq!(
            std::fs::metadata(&city).unwrap().modified().unwrap(),
            before
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
