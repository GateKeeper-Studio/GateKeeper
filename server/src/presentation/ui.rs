// Serves the dashboard (admin console + hosted login UI) as static files, when
// UI_DIR points at its build (`dashboard/build`, an adapter-static SPA).
// See docs/SERVING-THE-DASHBOARD-FROM-THE-SERVER.md.
//
// Three rules keep the UI from blurring into the API:
//   - the SPA entry page is served only on an explicit allow-list of paths, never as
//     a catch-all, so an unknown or mistyped API path still gets the API's JSON 404;
//   - documents carry their own CSP (scripts limited to this origin plus the hashes
//     of the entry page's inline bootstrap), while API responses keep
//     `default-src 'none'` from `routes::security_headers`;
//   - assets are mounted outside the per-IP rate limiter (see `routes::router`): a
//     cold page load fetches dozens of chunks, which must not eat the API budget of
//     every admin behind the same NAT.
//
// Nothing needs a restart after a rebuild: assets are read from disk per request,
// and the entry page (with the CSP derived from it) is re-read whenever its file
// changes. So `npm run build:watch` in the dashboard plus a browser refresh is the
// whole UI development loop; no Node server is involved.
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::SystemTime,
};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::Request,
    http::{HeaderValue, StatusCode, Uri, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use tower_http::services::{ServeDir, ServeFile};

// The paths that render the SPA. Everything else falls through to the API.
const SPA_ROUTES: &[&str] = &[
    "/",
    "/login",
    "/console",
    "/console/{*rest}",
    "/auth/{*rest}",
    "/terms",
    "/terms/{client_id}",
    "/privacy",
    "/privacy/{client_id}",
    // Legacy links; the SPA redirects them (`/dashboard/*` to `/console/*`, `/docs/*`
    // to the website).
    "/dashboard",
    "/dashboard/{*rest}",
    "/docs",
    "/docs/{*rest}",
];

// The built dashboard.
pub struct UiAssets {
    root: PathBuf,
    index_path: PathBuf,
    // The entry page and its policy, re-read when `index.html` changes on disk.
    entry: RwLock<EntryPage>,
}

#[derive(Clone)]
struct EntryPage {
    // What `index.html` looked like when read; `None` forces a re-read.
    stamp: Option<(SystemTime, u64)>,
    html: Bytes,
    csp: HeaderValue,
    // The appearance editor previews the hosted sign-in page in a same-origin
    // iframe (`/auth/{client}/sign-in?preview=1`); only that page may be framed,
    // and only by this origin.
    csp_preview: HeaderValue,
}

fn stamp_of(path: &Path) -> Option<(SystemTime, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

impl EntryPage {
    fn read(path: &Path) -> std::io::Result<Self> {
        let before = stamp_of(path);
        let html = std::fs::read(path)?;
        // A file still being written changes under us; remember no stamp then, so the
        // next request reads it again.
        let stamp = before.filter(|b| stamp_of(path).as_ref() == Some(b));
        let hashes = inline_script_hashes(&String::from_utf8_lossy(&html));
        Ok(Self {
            stamp,
            html: Bytes::from(html),
            csp: content_security_policy(&hashes, "'none'"),
            csp_preview: content_security_policy(&hashes, "'self'"),
        })
    }
}

impl UiAssets {
    // Fails when the directory is not a dashboard build, so a wrong UI_DIR stops the
    // boot instead of serving 404s.
    pub fn load(root: &Path) -> Result<Self, String> {
        let index_path = root.join("index.html");
        let entry = EntryPage::read(&index_path).map_err(|e| {
            format!(
                "UI_DIR: cannot read {} ({e}); build the dashboard first (npm run build)",
                index_path.display()
            )
        })?;
        Ok(Self {
            root: root.to_path_buf(),
            index_path,
            entry: RwLock::new(entry),
        })
    }

    // The entry page as it is on disk now. One `stat` per document request; the file
    // is only read again when it changed (a rebuild). While a rebuild has it missing
    // or half-written, the last good copy keeps being served.
    fn current_entry(&self) -> EntryPage {
        let cached = self.entry.read().expect("entry page lock").clone();
        let now = stamp_of(&self.index_path);
        if now.is_some() && now == cached.stamp {
            return cached;
        }
        match EntryPage::read(&self.index_path) {
            Ok(fresh) => {
                tracing::info!(
                    "dashboard rebuilt: serving the new {}",
                    self.index_path.display()
                );
                *self.entry.write().expect("entry page lock") = fresh.clone();
                fresh
            }
            Err(_) => cached,
        }
    }

    fn entry_page(&self, uri: &Uri) -> Response {
        let preview = uri.path().starts_with("/auth/")
            && uri
                .query()
                .is_some_and(|q| q.split('&').any(|pair| pair == "preview=1"));

        let entry = self.current_entry();
        let mut response = Response::new(Body::from(entry.html));
        let headers = response.headers_mut();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
        // The entry page names the current asset hashes, so it must never be cached
        // stale; the hashed assets themselves are immutable.
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            if preview {
                entry.csp_preview
            } else {
                entry.csp
            },
        );
        headers.insert(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static(if preview { "SAMEORIGIN" } else { "DENY" }),
        );
        response
    }
}

// The console and hosted-login documents' policy. Same directives as the former
// SvelteKit server, two of them tighter:
//   - `script-src` lists the inline bootstrap's hashes instead of 'unsafe-inline';
//   - `connect-src 'self'`: the login UI now calls the API on its own origin.
// `style-src` keeps 'unsafe-inline': Svelte and bits-ui set inline styles at runtime.
fn content_security_policy(script_hashes: &[String], frame_ancestors: &str) -> HeaderValue {
    let scripts = std::iter::once("'self'".to_string())
        .chain(script_hashes.iter().map(|h| format!("'{h}'")))
        .collect::<Vec<_>>()
        .join(" ");
    let policy = [
        "default-src 'self'".to_string(),
        format!("script-src {scripts}"),
        "style-src 'self' 'unsafe-inline'".to_string(),
        "img-src 'self' data: blob:".to_string(),
        "font-src 'self' data:".to_string(),
        "connect-src 'self'".to_string(),
        "object-src 'none'".to_string(),
        "base-uri 'self'".to_string(),
        format!("frame-ancestors {frame_ancestors}"),
        "form-action 'self'".to_string(),
    ]
    .join("; ");
    HeaderValue::from_str(&policy).expect("CSP is ASCII")
}

// `sha256-<base64>` of every inline `<script>` body in the page (those without
// `src`). The SvelteKit SPA entry page has exactly one: the hydration bootstrap.
fn inline_script_hashes(html: &str) -> Vec<String> {
    let mut hashes = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<script") {
        rest = &rest[start..];
        let Some(open_end) = rest.find('>') else {
            break;
        };
        let open_tag = &rest[..open_end];
        let body_start = open_end + 1;
        let Some(close) = rest[body_start..].find("</script>") else {
            break;
        };
        let body = &rest[body_start..body_start + close];
        if !open_tag.contains(" src=") && !body.trim().is_empty() {
            hashes.push(format!(
                "sha256-{}",
                STANDARD.encode(Sha256::digest(body.as_bytes()))
            ));
        }
        rest = &rest[body_start + close + "</script>".len()..];
    }
    hashes
}

// `/_app/immutable/*` is content-hashed by Vite, so browsers may keep it forever.
// Everything else under `/_app` (version.json, env.js) must be revalidated.
async fn asset_cache_headers(request: Request, next: Next) -> Response {
    let immutable = request.uri().path().starts_with("/_app/immutable/");
    let mut response = next.run(request).await;
    if response.status() == StatusCode::OK || response.status() == StatusCode::NOT_MODIFIED {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(if immutable {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            }),
        );
    }
    response
}

pub fn router<S: Clone + Send + Sync + 'static>(assets: Arc<UiAssets>) -> Router<S> {
    let app_dir = ServeDir::new(assets.root.join("_app"))
        .precompressed_br()
        .precompressed_gzip();
    let robots = ServeFile::new(assets.root.join("robots.txt"));

    let mut router = Router::new()
        .nest_service("/_app", app_dir)
        .route_service("/robots.txt", robots)
        .layer(middleware::from_fn(asset_cache_headers));

    for path in SPA_ROUTES {
        let assets = Arc::clone(&assets);
        router = router.route(
            path,
            get(move |uri: Uri| {
                let assets = Arc::clone(&assets);
                async move { assets.entry_page(&uri).into_response() }
            }),
        );
    }
    router
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use tower::ServiceExt;

    use super::*;

    #[test]
    fn only_inline_scripts_are_hashed() {
        let html = r#"<head><script type="module" src="/_app/a.js"></script>
            <link rel="modulepreload" href="/_app/b.js">
            <script>
                window.boot = 1;
            </script></head><body><script>  </script></body>"#;
        let hashes = inline_script_hashes(html);
        assert_eq!(hashes.len(), 1);
        let expected = STANDARD.encode(Sha256::digest(
            "\n                window.boot = 1;\n            ".as_bytes(),
        ));
        assert_eq!(hashes[0], format!("sha256-{expected}"));
    }

    #[test]
    fn the_policy_names_the_hashes_and_never_unsafe_inline_scripts() {
        let csp = content_security_policy(&["sha256-abc".to_string()], "'none'");
        let csp = csp.to_str().unwrap();
        assert!(csp.contains("script-src 'self' 'sha256-abc';"));
        assert!(csp.contains("connect-src 'self';"));
        assert!(csp.contains("frame-ancestors 'none';"));
        assert!(!csp.contains("script-src 'self' 'unsafe-inline'"));
    }

    // A minimal dashboard build in a fresh temporary directory.
    fn fake_build() -> PathBuf {
        let root = std::env::temp_dir().join(format!("gk-ui-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(root.join("_app/immutable/chunks")).unwrap();
        std::fs::write(
            root.join("index.html"),
            "<html><head><script>boot()</script></head><body></body></html>",
        )
        .unwrap();
        std::fs::write(root.join("_app/immutable/chunks/app.js"), "export {}").unwrap();
        std::fs::write(root.join("_app/version.json"), "{}").unwrap();
        std::fs::write(root.join("robots.txt"), "User-agent: *").unwrap();
        root
    }

    async fn get(router: &Router, uri: &str) -> Response {
        router
            .clone()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    fn header<'a>(response: &'a Response, name: header::HeaderName) -> &'a str {
        response
            .headers()
            .get(name)
            .map(|v| v.to_str().unwrap())
            .unwrap_or("")
    }

    #[test]
    fn a_directory_that_is_not_a_build_stops_the_boot() {
        let empty = std::env::temp_dir().join(format!("gk-ui-empty-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&empty).unwrap();
        let err = UiAssets::load(&empty).err().unwrap();
        assert!(err.contains("index.html"), "{err}");
    }

    #[tokio::test]
    async fn the_entry_page_is_served_only_on_spa_paths_with_its_own_policy() {
        let router: Router = router(Arc::new(UiAssets::load(&fake_build()).unwrap()));

        for path in [
            "/",
            "/login",
            "/console",
            "/console/tenants/abc/users",
            "/auth/my-client/sign-in?state=x",
            "/terms/my-client",
            "/dashboard/tenants",
        ] {
            let response = get(&router, path).await;
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert_eq!(header(&response, header::CACHE_CONTROL), "no-cache");
            assert_eq!(header(&response, header::X_FRAME_OPTIONS), "DENY");
            let csp = header(&response, header::CONTENT_SECURITY_POLICY);
            assert!(csp.contains("frame-ancestors 'none'"), "{path}: {csp}");
            let expected = STANDARD.encode(Sha256::digest(b"boot()"));
            assert!(csp.contains(&format!("'sha256-{expected}'")), "{csp}");
            let body = to_bytes(response.into_body(), 1 << 16).await.unwrap();
            assert!(body.starts_with(b"<html>"));
        }

        // API paths and unknown paths are not the SPA's: no route here, so the
        // API router (or its JSON 404) answers them.
        for path in [
            "/tenants/abc",
            "/admin/me",
            "/api/tenants",
            "/nope",
            "/v1/auth/login",
        ] {
            assert_eq!(
                get(&router, path).await.status(),
                StatusCode::NOT_FOUND,
                "{path}"
            );
        }
    }

    #[tokio::test]
    async fn a_rebuild_is_served_without_a_restart() {
        let root = fake_build();
        let router: Router = router(Arc::new(UiAssets::load(&root).unwrap()));
        let first = get(&router, "/console").await;
        let first_csp = header(&first, header::CONTENT_SECURITY_POLICY).to_string();

        // What `npm run build` does: a new entry page, with a new inline bootstrap.
        std::fs::write(
            root.join("index.html"),
            "<html><head><script>bootV2()</script></head><body>v2</body></html>",
        )
        .unwrap();

        let second = get(&router, "/console").await;
        let csp = header(&second, header::CONTENT_SECURITY_POLICY).to_string();
        let expected = STANDARD.encode(Sha256::digest(b"bootV2()"));
        assert!(csp.contains(&format!("'sha256-{expected}'")), "{csp}");
        assert_ne!(csp, first_csp);
        let body = to_bytes(second.into_body(), 1 << 16).await.unwrap();
        assert!(body.ends_with(b"v2</body></html>"));

        // Mid-rebuild the file is missing for a moment: keep serving the last copy.
        std::fs::remove_file(root.join("index.html")).unwrap();
        let during = get(&router, "/console").await;
        assert_eq!(during.status(), StatusCode::OK);
        let body = to_bytes(during.into_body(), 1 << 16).await.unwrap();
        assert!(body.ends_with(b"v2</body></html>"));
    }

    #[tokio::test]
    async fn only_the_hosted_sign_in_preview_may_be_framed_and_only_by_us() {
        let router: Router = router(Arc::new(UiAssets::load(&fake_build()).unwrap()));

        let preview = get(&router, "/auth/c/sign-in?preview=1").await;
        assert_eq!(header(&preview, header::X_FRAME_OPTIONS), "SAMEORIGIN");
        assert!(
            header(&preview, header::CONTENT_SECURITY_POLICY).contains("frame-ancestors 'self'")
        );

        let console = get(&router, "/console/tenants?preview=1").await;
        assert_eq!(header(&console, header::X_FRAME_OPTIONS), "DENY");
    }

    #[tokio::test]
    async fn hashed_assets_are_immutable_and_nothing_else_is() {
        let router: Router = router(Arc::new(UiAssets::load(&fake_build()).unwrap()));

        let chunk = get(&router, "/_app/immutable/chunks/app.js").await;
        assert_eq!(chunk.status(), StatusCode::OK);
        assert_eq!(
            header(&chunk, header::CACHE_CONTROL),
            "public, max-age=31536000, immutable"
        );
        assert!(header(&chunk, header::CONTENT_TYPE).contains("javascript"));

        let version = get(&router, "/_app/version.json").await;
        assert_eq!(header(&version, header::CACHE_CONTROL), "no-cache");

        assert_eq!(get(&router, "/robots.txt").await.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn assets_cannot_escape_the_build_directory() {
        let router: Router = router(Arc::new(UiAssets::load(&fake_build()).unwrap()));
        for path in [
            "/_app/../index.html",
            "/_app/..%2f..%2fetc%2fpasswd",
            "/_app/%2e%2e/%2e%2e/etc/passwd",
            "/_app/missing.js",
        ] {
            let status = get(&router, path).await.status();
            assert!(
                status == StatusCode::NOT_FOUND || status == StatusCode::BAD_REQUEST,
                "{path}: {status}"
            );
        }
    }
}
