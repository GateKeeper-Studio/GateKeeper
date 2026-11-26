// The optional HTTPS listener (`TLS_CERT_FILE` / `TLS_KEY_FILE`, see `TlsConfig`).
//
// Serves the same router as the HTTP listener, on its own port. Each connection is
// a TLS handshake (rustls, ring provider; TLS 1.2 and 1.3) and then HTTP/1.1 or
// HTTP/2 as negotiated by ALPN. The TCP peer address is attached to every request as
// `ConnectInfo`, exactly as `into_make_service_with_connect_info` does for plain HTTP,
// so client-IP resolution (`capture_client_context`, the rate limiter) works the
// same. On shutdown it stops accepting, then lets open connections finish (bounded).
use std::{future::Future, path::Path, sync::Arc, time::Duration};

use axum::{Router, body::Body, extract::ConnectInfo, http::Request};
use hyper::body::Incoming;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::{conn::auto::Builder, graceful::GracefulShutdown},
    service::TowerToHyperService,
};
use tokio::net::TcpListener;
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        self, ServerConfig,
        pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    },
};
use tower::ServiceExt;

// A client that connects and never finishes the handshake must not hold a task.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
// How long open connections get to finish after a shutdown signal.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(10);

// The rustls configuration from PEM files. An error names the file and the problem.
pub fn load_server_config(cert_file: &Path, key_file: &Path) -> Result<ServerConfig, String> {
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(cert_file)
        .and_then(|certs| certs.collect())
        .map_err(|e| format!("{}: {e}", cert_file.display()))?;
    if certs.is_empty() {
        return Err(format!("{}: no certificate found", cert_file.display()));
    }
    let key = PrivateKeyDer::from_pem_file(key_file)
        .map_err(|e| format!("{}: {e}", key_file.display()))?;

    let mut config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|e| e.to_string())?
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| format!("certificate and key do not match: {e}"))?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

// Accepts TLS connections on `listener` until `shutdown` resolves.
pub async fn serve(
    listener: TcpListener,
    config: ServerConfig,
    app: Router,
    shutdown: impl Future<Output = ()>,
) {
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let builder = Builder::new(TokioExecutor::new());
    let graceful = GracefulShutdown::new();
    tokio::pin!(shutdown);

    loop {
        let (tcp, peer) = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok(accepted) => accepted,
                Err(err) => {
                    // Typically EMFILE: back off instead of spinning.
                    tracing::warn!("https accept failed: {err}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            },
            () = &mut shutdown => break,
        };

        let acceptor = acceptor.clone();
        let builder = builder.clone();
        let app = app.clone();
        let watcher = graceful.watcher();
        tokio::spawn(async move {
            let tls = match tokio::time::timeout(HANDSHAKE_TIMEOUT, acceptor.accept(tcp)).await {
                Ok(Ok(tls)) => tls,
                Ok(Err(err)) => {
                    tracing::debug!("tls handshake with {peer} failed: {err}");
                    return;
                }
                Err(_) => return,
            };
            let service = app.map_request(move |request: Request<Incoming>| {
                let mut request = request.map(Body::new);
                request.extensions_mut().insert(ConnectInfo(peer));
                request
            });
            let connection = builder.serve_connection_with_upgrades(
                TokioIo::new(tls),
                TowerToHyperService::new(service),
            );
            if let Err(err) = watcher.watch(connection.into_owned()).await {
                tracing::debug!("https connection with {peer} ended: {err}");
            }
        });
    }

    drop(listener);
    tokio::select! {
        () = graceful.shutdown() => {}
        () = tokio::time::sleep(DRAIN_TIMEOUT) => {
            tracing::warn!("https connections still open after {DRAIN_TIMEOUT:?}; closing");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_files_are_reported_by_name() {
        let err = load_server_config(
            Path::new("/nonexistent/cert.pem"),
            Path::new("/nonexistent/key.pem"),
        )
        .unwrap_err();
        assert!(err.contains("/nonexistent/cert.pem"), "{err}");
    }
}
