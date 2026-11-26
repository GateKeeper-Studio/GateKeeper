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

// The console session a bridged `/api` request runs under (its row id, never the
// cookie value). Placed in the request extensions by `console_api_bridge`, so a
// handler that ends the admin's other sessions can keep this one. Absent on bearer
// requests.
#[derive(Debug, Clone, Copy)]
pub struct ConsoleSessionId(pub uuid::Uuid);

// How the console session behaves on this deployment.
#[derive(Debug, Clone)]
pub struct ConsoleSettings {
    // scheme://host[:port] of AUTH_UI_BASE_URL, which is this server's own public
    // origin when it serves the UI. The links the server hands out (QR sign-in,
    // admin invitations) point here.
    pub public_origin: String,
    // Every origin allowed to drive the session; the public origin comes first.
    // Production has only that one. Development also has this server's plain-HTTP
    // loopback listener (`also_at`), so the console keeps working on localhost when
    // AUTH_UI_BASE_URL is a LAN or TLS address for phones.
    origins: Vec<ConsoleOrigin>,
}

#[derive(Debug, Clone)]
struct ConsoleOrigin {
    origin: String,
    // host[:port] of `origin`, compared with the request's `Host` header.
    authority: String,
    // True for https: the cookie is then `Secure` and `__Host-`.
    secure: bool,
}

impl ConsoleOrigin {
    fn new(url: &str) -> Self {
        let origin = origin_of(url);
        let authority = origin
            .split_once("://")
            .map_or(origin.as_str(), |(_, authority)| authority)
            .to_string();
        Self {
            secure: origin.starts_with("https://"),
            authority,
            origin,
        }
    }

    fn cookie_name(&self) -> &'static str {
        if self.secure {
            SECURE_COOKIE
        } else {
            DEV_COOKIE
        }
    }
}

impl ConsoleSettings {
    pub fn from_public_url(url: &str) -> Self {
        let public = ConsoleOrigin::new(url);
        Self {
            public_origin: public.origin.clone(),
            origins: vec![public],
        }
    }

    // Lets the console also run at these URLs' origins: other addresses of this
    // same server. Development only (bootstrap adds the loopback listener); each
    // gets the cookie its scheme allows.
    pub fn also_at<I, S>(mut self, urls: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for url in urls {
            let extra = ConsoleOrigin::new(url.as_ref());
            if !self.accepts_origin(&extra.origin) {
                self.origins.push(extra);
            }
        }
        self
    }

    pub fn accepts_origin(&self, origin: &str) -> bool {
        self.origins.iter().any(|o| o.origin == origin)
    }

    // The console origin this request was made at: the one its `Host` names, else
    // the public origin. A browser sets `Host` itself, so a page cannot pick another
    // origin's cookie or Origin check. A request whose `Host` names none of them
    // (HTTP/2 sends `:authority` instead, a proxy may rewrite it) falls back to the
    // public origin, which with a single origin is the only behavior there is.
    fn origin_for(&self, headers: &HeaderMap) -> &ConsoleOrigin {
        let host = headers.get(header::HOST).and_then(|h| h.to_str().ok());
        host.and_then(|host| {
            self.origins[1..]
                .iter()
                .find(|o| o.authority.eq_ignore_ascii_case(host.trim()))
        })
        .unwrap_or(&self.origins[0])
    }

    // `Set-Cookie` opening a session. SameSite=Strict costs the console nothing: its
    // pages are public static files and every authenticated call is a same-origin
    // fetch, so the cookie is never needed on a cross-site navigation.
    pub fn session_cookie(
        &self,
        headers: &HeaderMap,
        value: &str,
        max_age_seconds: i64,
    ) -> HeaderValue {
        let origin = self.origin_for(headers);
        let secure = if origin.secure { "; Secure" } else { "" };
        HeaderValue::from_str(&format!(
            "{}={value}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age_seconds}{secure}",
            origin.cookie_name()
        ))
        .expect("an opaque token is a valid header value")
    }

    pub fn cleared_cookie(&self, headers: &HeaderMap) -> HeaderValue {
        let origin = self.origin_for(headers);
        let secure = if origin.secure { "; Secure" } else { "" };
        HeaderValue::from_str(&format!(
            "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{secure}",
            origin.cookie_name()
        ))
        .expect("static cookie is a valid header value")
    }

    // The session cookie's value, if the request carries one.
    pub fn read_cookie(&self, headers: &HeaderMap) -> Option<String> {
        let name = self.origin_for(headers).cookie_name();
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
    //   - any state-changing method must carry an `Origin` equal to the console
    //     origin the request was made at.
    pub fn refuse_cross_origin(&self, method: &Method, headers: &HeaderMap) -> Option<Response> {
        if let Some(site) = headers.get("sec-fetch-site")
            && site.as_bytes() != b"same-origin"
        {
            return Some(cross_origin_refused());
        }
        let safe = matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS);
        if !safe {
            let origin = headers.get(header::ORIGIN).and_then(|o| o.to_str().ok());
            if origin != Some(self.origin_for(headers).origin.as_str()) {
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
    let (access_token, session_id) = match resolved {
        Ok(resolved) => (resolved.access_token, resolved.session_id),
        Err(DomainError::AuthenticationError(_)) => {
            let mut response =
                ApiError::Unauthorized("session expired".to_string()).into_response();
            response.headers_mut().insert(
                header::SET_COOKIE,
                console.cleared_cookie(request.headers()),
            );
            return response;
        }
        Err(other) => return ApiError::from(other).into_response(),
    };

    let bearer = HeaderValue::from_str(&format!("Bearer {access_token}"))
        .expect("a JWT is a valid header value");
    request.headers_mut().insert(header::AUTHORIZATION, bearer);
    // The cookie has done its job; the admin handlers never need it.
    request.headers_mut().remove(header::COOKIE);
    request
        .extensions_mut()
        .insert(ConsoleSessionId(session_id));

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
        let cookie = secure.session_cookie(&HeaderMap::new(), "abc", 60);
        assert_eq!(
            cookie.to_str().unwrap(),
            "__Host-gk_admin=abc; Path=/; HttpOnly; SameSite=Strict; Max-Age=60; Secure"
        );

        let dev = settings("http://localhost:5173");
        assert_eq!(
            dev.session_cookie(&HeaderMap::new(), "abc", 60)
                .to_str()
                .unwrap(),
            "gk_admin=abc; Path=/; HttpOnly; SameSite=Strict; Max-Age=60"
        );
    }

    // Development: AUTH_UI_BASE_URL is the TLS address phones use, and the console
    // is also opened on the loopback HTTP listener.
    fn dev_with_loopback() -> ConsoleSettings {
        settings("https://192-168-0-140.sslip.io:8443")
            .also_at(["http://localhost:8080", "http://127.0.0.1:8080"])
    }

    #[test]
    fn an_extra_origin_is_served_at_its_own_host_with_the_cookie_its_scheme_allows() {
        let s = dev_with_loopback();
        let local = headers(&[
            ("host", "localhost:8080"),
            ("origin", "http://localhost:8080"),
            ("sec-fetch-site", "same-origin"),
        ]);
        assert!(s.refuse_cross_origin(&Method::POST, &local).is_none());
        assert_eq!(
            s.session_cookie(&local, "abc", 60).to_str().unwrap(),
            "gk_admin=abc; Path=/; HttpOnly; SameSite=Strict; Max-Age=60"
        );
        assert_eq!(
            s.cleared_cookie(&local).to_str().unwrap(),
            "gk_admin=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"
        );
        let local_get = headers(&[("host", "localhost:8080"), ("cookie", "gk_admin=local")]);
        assert_eq!(s.read_cookie(&local_get).as_deref(), Some("local"));

        // The public origin keeps its prefixed cookie; HTTP/2 sends no `Host`.
        let public = headers(&[
            ("origin", "https://192-168-0-140.sslip.io:8443"),
            ("cookie", "gk_admin=wrong-name; __Host-gk_admin=public"),
        ]);
        assert!(s.refuse_cross_origin(&Method::POST, &public).is_none());
        assert!(
            s.session_cookie(&public, "abc", 60)
                .to_str()
                .unwrap()
                .starts_with("__Host-gk_admin=abc;")
        );
        assert_eq!(s.read_cookie(&public).as_deref(), Some("public"));
        assert_eq!(s.public_origin, "https://192-168-0-140.sslip.io:8443");
    }

    #[test]
    fn each_origin_accepts_only_its_own_requests() {
        let s = dev_with_loopback();
        // A page on one allowed origin cannot write through another.
        let mismatched = headers(&[
            ("host", "localhost:8080"),
            ("origin", "https://192-168-0-140.sslip.io:8443"),
        ]);
        assert!(s.refuse_cross_origin(&Method::POST, &mismatched).is_some());
        let mismatched = headers(&[
            ("host", "192-168-0-140.sslip.io:8443"),
            ("origin", "http://localhost:8080"),
        ]);
        assert!(s.refuse_cross_origin(&Method::POST, &mismatched).is_some());
        // An origin the server never added stays refused.
        let lan = headers(&[
            ("host", "192.168.0.140:8080"),
            ("origin", "http://192.168.0.140:8080"),
        ]);
        assert!(s.refuse_cross_origin(&Method::POST, &lan).is_some());
    }

    #[test]
    fn without_extra_origins_the_host_header_changes_nothing() {
        let s = settings("https://id.example.com");
        let local = headers(&[
            ("host", "localhost:8080"),
            ("origin", "http://localhost:8080"),
        ]);
        assert!(s.refuse_cross_origin(&Method::POST, &local).is_some());
        assert!(
            s.session_cookie(&local, "abc", 60)
                .to_str()
                .unwrap()
                .starts_with("__Host-gk_admin=abc;")
        );
        assert!(!s.accepts_origin("http://localhost:8080"));
        assert!(dev_with_loopback().accepts_origin("http://127.0.0.1:8080"));
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
