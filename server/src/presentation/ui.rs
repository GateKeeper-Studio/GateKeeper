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
//
// First paint. The build is one `index.html` for every route, which on its own names
// only the router's entry modules: the browser would discover a page's ~50 modules an
// import wave at a time, a round trip per wave, with nothing on screen. So for each
// request the entry page is completed with what server rendering used to provide
// (`with_route_hints`):
//   - `<link rel="modulepreload">` for every module the route needs, from the
//     `_app/route-preloads.json` the dashboard build writes, so they load in parallel;
//   - on hosted login pages, `data-boot="auth"`, which shows the sign-in skeleton
//     that `app.html` paints before any JavaScript runs;
//   - on hosted login pages, a preload of the client settings the page fetches first,
//     so that request runs alongside the modules instead of after them.
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::SystemTime,
};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::Request,
    http::{HeaderMap, HeaderValue, StatusCode, Uri, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tower_http::services::{ServeDir, ServeFile};

// The paths that render the SPA. Everything else falls through to the API.
const SPA_ROUTES: &[&str] = &[
    "/",
    "/login",
    // Accepting an admin invitation, and the two-step setup an organization can
    // require before the console opens.
    "/login/{*rest}",
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
    // Per-route preload hints, read with the page (a rebuild changes both).
    hints: Arc<RouteHints>,
}

// `_app/route-preloads.json`, written by `dashboard/scripts/route-preloads.mjs`.
#[derive(Deserialize, Default)]
struct RoutePreloadsFile {
    #[serde(default)]
    routes: Vec<RoutePreloadEntry>,
    i18n: Option<I18nSettings>,
}

#[derive(Deserialize)]
struct RoutePreloadEntry {
    pattern: String,
    #[serde(default)]
    js: Vec<String>,
    #[serde(default)]
    css: Vec<String>,
}

// How the dashboard picks the locale a page renders in: its cookie, else the base
// locale (the paraglide strategy `["cookie", "globalVariable", "baseLocale"]`).
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct I18nSettings {
    locales: Vec<String>,
    base_locale: String,
    cookie_name: String,
}

#[derive(Default)]
struct RouteHints {
    // In the build's route order (most specific first), like SvelteKit's router.
    routes: Vec<RouteHint>,
    i18n: Option<I18nSettings>,
}

struct RouteHint {
    segments: Vec<Segment>,
    // The ready-made `<link>` tags for this route.
    tags: String,
}

#[derive(Debug, PartialEq)]
enum Segment {
    Literal(String),
    Param,
    Rest,
}

// A SvelteKit route id as path segments: `(group)` segments do not appear in URLs,
// `[...rest]` matches any number of segments, `[param]` exactly one.
fn route_segments(pattern: &str) -> Vec<Segment> {
    pattern
        .split('/')
        .filter(|s| !(s.is_empty() || (s.starts_with('(') && s.ends_with(')'))))
        .map(|s| {
            if s.starts_with("[...") {
                Segment::Rest
            } else if s.starts_with('[') {
                Segment::Param
            } else {
                Segment::Literal(s.to_string())
            }
        })
        .collect()
}

fn route_matches(pattern: &[Segment], path: &[&str]) -> bool {
    match (pattern.first(), path.first()) {
        (None, None) => true,
        (Some(Segment::Rest), _) => {
            (0..=path.len()).any(|skip| route_matches(&pattern[1..], &path[skip..]))
        }
        (Some(Segment::Param), Some(_)) => route_matches(&pattern[1..], &path[1..]),
        (Some(Segment::Literal(lit)), Some(seg)) if lit == seg => {
            route_matches(&pattern[1..], &path[1..])
        }
        _ => false,
    }
}

impl RouteHints {
    // `html` is the entry page: modules it already preloads are not repeated.
    fn read(root: &Path, html: &str) -> Self {
        let path = root.join("_app").join("route-preloads.json");
        let file: RoutePreloadsFile = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                tracing::warn!(
                    "{}: unreadable ({e}); pages load without preload hints",
                    path.display()
                );
                RoutePreloadsFile::default()
            }),
            // An older build without the file: no hints, nothing else changes.
            Err(_) => RoutePreloadsFile::default(),
        };
        let routes = file
            .routes
            .into_iter()
            .map(|route| {
                let mut tags = String::new();
                for href in route
                    .js
                    .iter()
                    .filter(|href| safe_href(href) && !html.contains(href.as_str()))
                {
                    tags.push_str(&format!(r#"<link rel="modulepreload" href="{href}">"#));
                }
                for href in route
                    .css
                    .iter()
                    .filter(|href| safe_href(href) && !html.contains(href.as_str()))
                {
                    tags.push_str(&format!(r#"<link rel="preload" as="style" href="{href}">"#));
                }
                RouteHint {
                    segments: route_segments(&route.pattern),
                    tags,
                }
            })
            .collect();
        Self {
            routes,
            i18n: file.i18n,
        }
    }

    fn for_path(&self, path: &str) -> Option<&RouteHint> {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        self.routes
            .iter()
            .find(|route| route_matches(&route.segments, &segments))
    }

    // The locale the page will render in, as the dashboard decides it.
    fn locale(&self, headers: &HeaderMap) -> Option<String> {
        let i18n = self.i18n.as_ref()?;
        let from_cookie = headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(name, _)| *name == i18n.cookie_name)
            .and_then(|(_, value)| {
                i18n.locales
                    .iter()
                    .find(|locale| locale.eq_ignore_ascii_case(value.trim()))
            });
        Some(from_cookie.unwrap_or(&i18n.base_locale).clone())
    }
}

// Only build-relative asset paths are turned into tags (the file is ours, but it is
// read from disk; nothing in it should be able to break out of an attribute).
fn safe_href(href: &str) -> bool {
    href.starts_with("/_app/")
        && href
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
}

// A client id as it may appear in the preloaded URL.
fn safe_client_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

// The request a hosted login page makes first (`getClientAuthData` in the dashboard),
// for `/auth/{client_id}/...`. Must stay byte-identical to the URL the page builds,
// or the browser fetches it a second time.
fn client_auth_data_url(path: &str, locale: &str) -> Option<String> {
    let client_id = path.strip_prefix("/auth/")?.split('/').next()?;
    safe_client_id(client_id)
        .then(|| format!("/v1/auth/client/{client_id}/auth-data?locale={locale}"))
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
        let text = String::from_utf8_lossy(&html);
        let hashes = inline_script_hashes(&text);
        let root = path.parent().unwrap_or(Path::new("."));
        let hints = Arc::new(RouteHints::read(root, &text));
        Ok(Self {
            stamp,
            html: Bytes::from(html),
            csp: content_security_policy(&hashes, "'none'"),
            csp_preview: content_security_policy(&hashes, "'self'"),
            hints,
        })
    }

    // The entry page completed for one request (see the module comment). Only adds
    // `<link>` tags and an attribute: the inline scripts, and so the CSP hashes, are
    // untouched.
    fn with_route_hints(&self, path: &str, headers: &HeaderMap) -> Bytes {
        let auth_page = path.starts_with("/auth/");
        let route_tags = self.hints.for_path(path).map(|route| route.tags.as_str());
        let data_tag = auth_page
            .then(|| self.hints.locale(headers))
            .flatten()
            .and_then(|locale| client_auth_data_url(path, &locale))
            .map(|url| format!(r#"<link rel="preload" as="fetch" href="{url}" crossorigin>"#));
        if route_tags.is_none_or(str::is_empty) && data_tag.is_none() && !auth_page {
            return self.html.clone();
        }

        let html = String::from_utf8_lossy(&self.html);
        let mut out = String::with_capacity(html.len() + 4096);
        let (head, rest) = html.split_at(html.find("</head>").unwrap_or(html.len()));
        if auth_page {
            out.push_str(&head.replacen("<html", r#"<html data-boot="auth""#, 1));
        } else {
            out.push_str(head);
        }
        if let Some(tag) = &data_tag {
            out.push_str(tag);
        }
        out.push_str(route_tags.unwrap_or_default());
        out.push_str(rest);
        Bytes::from(out)
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

    fn entry_page(&self, uri: &Uri, request_headers: &HeaderMap) -> Response {
        let preview = uri.path().starts_with("/auth/")
            && uri
                .query()
                .is_some_and(|q| q.split('&').any(|pair| pair == "preview=1"));

        let entry = self.current_entry();
        let html = entry.with_route_hints(uri.path(), request_headers);
        let mut response = Response::new(Body::from(html));
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

// Where the console's maps load their base map from: the Carto style JSON, then its
// vector tiles, sprites and glyphs. Fetched with `fetch`, so they belong in
// `connect-src` (MapLibre turns the images into bitmaps itself; `img-src` already has
// `blob:`). A different base map means changing this list and `defaultStyles` in
// `dashboard/src/lib/components/ui/map/map.svelte` together.
macro_rules! map_tile_origins {
    () => {
        "https://basemaps.cartocdn.com https://tiles.basemaps.cartocdn.com"
    };
}
const MAP_TILE_ORIGINS: &str = map_tile_origins!();

// Script assets carry a policy of their own. It has no effect on a `<script>` (the
// document's policy governs those) but it is THE policy of a dedicated worker, which
// takes its CSP from the response that delivered its script: MapLibre's worker
// (`maplibre-gl-worker.*.mjs`) fetches the tiles. Without it the app-wide
// `default-src 'none'` would apply and the map would stay blank.
const WORKER_SCRIPT_CSP: &str = concat!(
    "default-src 'none'; connect-src 'self' ",
    map_tile_origins!()
);

// The console and hosted-login documents' policy. Same directives as the former
// SvelteKit server, two of them tighter:
//   - `script-src` lists the inline bootstrap's hashes instead of 'unsafe-inline';
//   - `connect-src` is this origin (the login UI calls the API on its own origin) plus
//     the map tiles, and nothing else. The hosted login pages never draw a map;
//     they share the policy because one entry page serves both.
// `style-src` keeps 'unsafe-inline': Svelte and bits-ui set inline styles at runtime.
// Workers fall back to `script-src`: MapLibre's is a same-origin module script.
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
        format!("connect-src 'self' {MAP_TILE_ORIGINS}"),
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
// Everything else under `/_app` (version.json, env.js) must be revalidated. Scripts
// also get `WORKER_SCRIPT_CSP`; the app-wide headers layer only fills in a CSP that is
// missing, so this one stays.
async fn asset_cache_headers(request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let immutable = path.starts_with("/_app/immutable/");
    let script = path.ends_with(".js") || path.ends_with(".mjs");
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
        if script {
            response.headers_mut().insert(
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static(WORKER_SCRIPT_CSP),
            );
        }
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
            get(move |uri: Uri, headers: HeaderMap| {
                let assets = Arc::clone(&assets);
                async move { assets.entry_page(&uri, &headers).into_response() }
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
        assert!(csp.contains(&format!("connect-src 'self' {MAP_TILE_ORIGINS};")));
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

    fn header(response: &Response, name: header::HeaderName) -> &str {
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
            "/login/invite",
            "/login/two-step",
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
            // Hosted login pages also carry `data-boot="auth"` (the skeleton).
            assert!(body.starts_with(b"<html"), "{path}");
            assert!(body.ends_with(b"</html>"), "{path}");
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
        // A script's own policy is the one it runs under as a worker.
        assert_eq!(
            header(&chunk, header::CONTENT_SECURITY_POLICY),
            WORKER_SCRIPT_CSP
        );

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

    #[test]
    fn route_ids_match_like_the_sveltekit_router() {
        let sign_in = route_segments("/auth/[clientId]/sign-in");
        assert!(route_matches(&sign_in, &["auth", "abc", "sign-in"]));
        assert!(!route_matches(&sign_in, &["auth", "abc", "sign-up"]));
        assert!(!route_matches(&sign_in, &["auth", "abc"]));

        let rest = route_segments("/(app)/console/[...path]");
        assert_eq!(rest, [Segment::Literal("console".into()), Segment::Rest]);
        assert!(route_matches(&rest, &["console"]));
        assert!(route_matches(&rest, &["console", "tenants", "x", "users"]));
        assert!(!route_matches(&rest, &["login"]));
    }

    fn build_with_hints() -> PathBuf {
        let root = fake_build();
        std::fs::write(
            root.join("index.html"),
            r#"<html lang="en"><head><script>boot()</script><link href="/_app/immutable/entry/start.js" rel="modulepreload"></head><body></body></html>"#,
        )
        .unwrap();
        std::fs::write(
            root.join("_app/route-preloads.json"),
            r#"{"routes":[
                {"pattern":"/auth/[clientId]/sign-in","js":["/_app/immutable/entry/start.js","/_app/immutable/nodes/13.js","/_app/x\"><script>"],"css":["/_app/immutable/assets/13.css"]},
                {"pattern":"/console","js":["/_app/immutable/nodes/20.js"],"css":[]}
               ],
               "i18n":{"locales":["en","es","pt-br"],"baseLocale":"en","cookieName":"PARAGLIDE_LOCALE"}}"#,
        )
        .unwrap();
        root
    }

    async fn page_html(router: &Router, uri: &str, cookie: Option<&str>) -> (String, String) {
        let mut request = Request::get(uri);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let response = router
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let csp = header(&response, header::CONTENT_SECURITY_POLICY).to_string();
        let body = to_bytes(response.into_body(), 1 << 16).await.unwrap();
        (String::from_utf8(body.to_vec()).unwrap(), csp)
    }

    #[tokio::test]
    async fn the_entry_page_names_what_the_route_needs_before_any_script_runs() {
        let router: Router = router(Arc::new(UiAssets::load(&build_with_hints()).unwrap()));

        let (html, csp) = page_html(
            &router,
            "/auth/client-1/sign-in?state=s",
            Some("theme=x; PARAGLIDE_LOCALE=PT-BR"),
        )
        .await;
        assert!(
            html.starts_with(r#"<html data-boot="auth" lang="en">"#),
            "{html}"
        );
        assert!(html.contains(r#"<link rel="modulepreload" href="/_app/immutable/nodes/13.js">"#));
        assert!(
            html.contains(
                r#"<link rel="preload" as="style" href="/_app/immutable/assets/13.css">"#
            )
        );
        // The client settings the page fetches first, in the locale it will render in.
        assert!(html.contains(
            r#"<link rel="preload" as="fetch" href="/v1/auth/client/client-1/auth-data?locale=pt-br" crossorigin>"#
        ));
        // Not repeated when the page already preloads it; nothing unsafe is emitted.
        assert_eq!(html.matches("/_app/immutable/entry/start.js").count(), 1);
        assert!(!html.contains("<script>\""), "{html}");
        // The hints are links only: the CSP still names the one inline script.
        let expected = STANDARD.encode(Sha256::digest(b"boot()"));
        assert!(csp.contains(&format!("'sha256-{expected}'")), "{csp}");

        // No locale cookie: the base locale. An odd client id: no data hint.
        let (html, _) = page_html(&router, "/auth/client-1/sign-in", None).await;
        assert!(html.contains("auth-data?locale=en"));
        let (html, _) = page_html(&router, "/auth/a%22b/sign-in", None).await;
        assert!(!html.contains("auth-data"));

        // The console gets its own modules, and no login skeleton.
        let (html, _) = page_html(&router, "/console", None).await;
        assert!(html.contains("/_app/immutable/nodes/20.js"));
        assert!(!html.contains("data-boot"));
        assert!(!html.contains("nodes/13.js"));
    }
}
