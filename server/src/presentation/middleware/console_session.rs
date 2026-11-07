// The admin console's browser session: cookie handling, the cross-site request
// guard, and the `/api` bridge that turns the cookie into a bearer token.
//
// When this server serves the dashboard (UI_DIR), the console never holds a token.
// It holds an httpOnly cookie, and calls the admin API under `/api/*` on its own
// origin. The bridge resolves the cookie to a live session, mints a short-lived
// admin access token, and hands the request to the unchanged admin router, where
// `require_admin` verifies that token like any other. A cookie therefore only ever
// opens the admin surface, and only from this origin.
// See docs/SERVING-THE-DASHBOARD-FROM-THE-SERVER.md.
use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    application::use_cases::admin_web_session::resolve,
    domain::errors::DomainError,
    presentation::{errors::ApiError, routes::AppState},
};

// Secure deployments use the `__Host-` prefix: the browser then refuses the cookie
// unless it is Secure, has Path=/ and no Domain, so a sibling subdomain can neither
// set nor shadow it. Plain-http development cannot use the prefix (it requires
// Secure), so it gets an unprefixed, non-Secure cookie instead.
const SECURE_COOKIE: &str = "__Host-gk_admin";
const DEV_COOKIE: &str = "gk_admin";

// How the console session behaves on this deployment.
#[derive(Debug, Clone)]
pub struct ConsoleSettings {
    // The only origin allowed to drive the session: scheme://host[:port] of
    // AUTH_UI_BASE_URL, which is this server's own public origin when it serves the UI.
    pub public_origin: String,
    // True when the public origin is https: the cookie is then `Secure` and `__Host-`.
    pub secure_cookie: bool,
}

impl ConsoleSettings {
    pub fn from_public_url(url: &str) -> Self {
        let public_origin = origin_of(url);
        Self {
            secure_cookie: public_origin.starts_with("https://"),
            public_origin,
        }
    }

    pub fn cookie_name(&self) -> &'static str {
        if self.secure_cookie {
            SECURE_COOKIE
        } else {
            DEV_COOKIE
        }
    }

    // `Set-Cookie` opening a session. SameSite=Strict costs the console nothing: its
    // pages are public static files and every authenticated call is a same-origin
    // fetch, so the cookie is never needed on a cross-site navigation.
    pub fn session_cookie(&self, value: &str, max_age_seconds: i64) -> HeaderValue {
        let secure = if self.secure_cookie { "; Secure" } else { "" };
        HeaderValue::from_str(&format!(
            "{}={value}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age_seconds}{secure}",
            self.cookie_name()
        ))
        .expect("an opaque token is a valid header value")
    }

    pub fn cleared_cookie(&self) -> HeaderValue {
        let secure = if self.secure_cookie { "; Secure" } else { "" };
        HeaderValue::from_str(&format!(
            "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{secure}",
            self.cookie_name()
        ))
        .expect("static cookie is a valid header value")
    }

    // The session cookie's value, if the request carries one.
    pub fn read_cookie(&self, headers: &HeaderMap) -> Option<String> {
        let name = self.cookie_name();
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    // Refuses a request a browser sent on behalf of another site (CSRF).
    //
    // SameSite=Strict already keeps the cookie off cross-site requests, but "site"
    // is the registrable domain: a relying party on a sibling subdomain is
    // same-site. So the origin is checked too:
    //   - `Sec-Fetch-Site`, when the browser sends it, must be `same-origin`;
    //   - any state-changing method must carry an `Origin` equal to ours.
    pub fn refuse_cross_origin(&self, method: &Method, headers: &HeaderMap) -> Option<Response> {
        if let Some(site) = headers.get("sec-fetch-site") {
            if site.as_bytes() != b"same-origin" {
                return Some(cross_origin_refused());
            }
        }
        let safe = matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS);
        if !safe {
            let origin = headers.get(header::ORIGIN).and_then(|o| o.to_str().ok());
            if origin != Some(self.public_origin.as_str()) {
                return Some(cross_origin_refused());
            }
        }
        None
    }
}

fn cross_origin_refused() -> Response {
    ApiError::Forbidden("cross-origin request refused".to_string()).into_response()
}

// scheme://host[:port] of a URL; the path, query and fragment are dropped.
pub fn origin_of(url: &str) -> String {
    let url = url.trim();
    match url.find("://") {
        Some(scheme_end) => {
            let rest = &url[scheme_end + 3..];
            let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            format!(
                "{}://{}",
                url[..scheme_end].to_ascii_lowercase(),
                rest[..authority_end].to_ascii_lowercase()
            )
        }
        None => url.trim_end_matches('/').to_string(),
    }
}

// Middleware on the admin router nested under `/api`: cookie in, bearer token out.
pub async fn console_api_bridge(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let console = &state.console;

    if let Some(refused) = console.refuse_cross_origin(request.method(), request.headers()) {
        return refused;
    }

    // Whatever bearer token the caller sent is not what authenticates this path; the
    // session is. Dropping it means a `/api` request is never authorized by anything
    // but the cookie.
    request.headers_mut().remove(header::AUTHORIZATION);

    let Some(session_token) = console.read_cookie(request.headers()) else {
        return ApiError::Unauthorized("not signed in".to_string()).into_response();
    };

    let resolved = state
        .admin
        .console_resolve
        .execute(resolve::query::Query { session_token })
        .await;
    let access_token = match resolved {
        Ok(resolved) => resolved.access_token,
        Err(DomainError::AuthenticationError(_)) => {
            let mut response =
                ApiError::Unauthorized("session expired".to_string()).into_response();
            response
                .headers_mut()
                .insert(header::SET_COOKIE, console.cleared_cookie());
            return response;
        }
        Err(other) => return ApiError::from(other).into_response(),
    };

    let bearer = HeaderValue::from_str(&format!("Bearer {access_token}"))
        .expect("a JWT is a valid header value");
    request.headers_mut().insert(header::AUTHORIZATION, bearer);
    // The cookie has done its job; the admin handlers never need it.
    request.headers_mut().remove(header::COOKIE);

    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(url: &str) -> ConsoleSettings {
        ConsoleSettings::from_public_url(url)
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (key, value) in pairs {
            map.append(
                axum::http::HeaderName::from_bytes(key.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        map
    }

    #[test]
    fn the_public_origin_drops_path_and_query() {
        assert_eq!(
            origin_of("https://ID.example.com/ui/?x=1"),
            "https://id.example.com"
        );
        assert_eq!(origin_of("http://localhost:8080"), "http://localhost:8080");
    }

    #[test]
    fn https_deployments_get_a_host_prefixed_secure_cookie() {
        let secure = settings("https://id.example.com");
        let cookie = secure.session_cookie("abc", 60);
        assert_eq!(
            cookie.to_str().unwrap(),
            "__Host-gk_admin=abc; Path=/; HttpOnly; SameSite=Strict; Max-Age=60; Secure"
        );

        let dev = settings("http://localhost:5173");
        assert_eq!(
            dev.session_cookie("abc", 60).to_str().unwrap(),
            "gk_admin=abc; Path=/; HttpOnly; SameSite=Strict; Max-Age=60"
        );
    }

    #[test]
    fn the_cookie_is_read_by_exact_name() {
        let s = settings("https://id.example.com");
        let h = headers(&[(
            "cookie",
            "x__Host-gk_admin=decoy; PARAGLIDE_LOCALE=en; __Host-gk_admin=real",
        )]);
        assert_eq!(s.read_cookie(&h).as_deref(), Some("real"));
        assert_eq!(s.read_cookie(&headers(&[("cookie", "other=1")])), None);
    }

    #[test]
    fn cross_origin_and_same_site_requests_are_refused() {
        let s = settings("https://id.example.com");
        let ok_post = headers(&[
            ("origin", "https://id.example.com"),
            ("sec-fetch-site", "same-origin"),
        ]);
        assert!(s.refuse_cross_origin(&Method::POST, &ok_post).is_none());
        assert!(
            s.refuse_cross_origin(&Method::GET, &HeaderMap::new())
                .is_none()
        );

        for refused in [
            headers(&[("origin", "https://evil.example")]),
            // A relying party on a sibling subdomain is same-SITE, not same-origin.
            headers(&[
                ("origin", "https://app.example.com"),
                ("sec-fetch-site", "same-site"),
            ]),
            // A state-changing request must say where it comes from.
            HeaderMap::new(),
        ] {
            assert!(s.refuse_cross_origin(&Method::DELETE, &refused).is_some());
        }
        let cross_site_get = headers(&[("sec-fetch-site", "cross-site")]);
        assert!(
            s.refuse_cross_origin(&Method::GET, &cross_site_get)
                .is_some()
        );
    }
}
