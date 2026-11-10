# Serving the dashboard from the Rust server: feasibility analysis

> **Status:** analysis written 2026-11-07 against `master` at `86298445`, then
> **implemented as an experiment on branch `feat/single-app-dashboard`**. See §0 for
> what was built and where it departs from this analysis.
> **Scope:** `dashboard/` (SvelteKit) and `server/` (Axum).

## 0. Implementation (branch `feat/single-app-dashboard`)

### Try it

```sh
# 1. Build the UI (writes dashboard/build).
cd dashboard && npm install && npm run build

# 2. Run the server with it (see "Switching back to master" below about the database).
cd ../server
UI_DIR=../dashboard/build AUTH_UI_BASE_URL=http://localhost:8080 OAUTH_ISSUER=http://localhost:8080 cargo run

# 3. Open http://localhost:8080/console (admin) or
#    http://localhost:8080/auth/<client_id>/sign-in?... (hosted login).
```

**Developing the UI.** Run `npm run build:watch` in `dashboard/`: it rebuilds `build/` on
every save. The server re-reads `index.html` (and its CSP hashes) when the file changes,
and reads assets from disk per request, so a browser refresh shows the change. No
restart is needed, and no Node server runs; Node is only the Svelte compiler.

For hot reload, `npm run dev` also still works: Vite runs on :5173, and the server needs
`AUTH_UI_BASE_URL=http://localhost:5173`.

**First paint without a Node server.** A static SPA starts as an empty page: the
browser discovers a route's ~50 modules one import wave at a time, then fetches its
data, with nothing on screen (measured on a hosted sign-in page, slow 4G: blank for
2.4 s). Three things replace what SvelteKit's server rendering provided:

1. `app.html` paints a skeleton with the HTML: a sign-in card on hosted login pages
   (the server marks them `data-boot="auth"`), a spinner elsewhere, in the saved
   light/dark mode (a small inline script, CSP-hashed like the bootstrap). The root
   layout removes it when the app has mounted.
2. The dashboard build writes `_app/route-preloads.json` (route → modules + CSS), and
   `ui.rs` adds `<link rel="modulepreload">` tags for the requested route, so all its
   modules are requested at once.
3. On `/auth/{client}/…`, `ui.rs` also preloads `/v1/auth/client/{client}/auth-data`
   in the locale the page will render in (the `PARAGLIDE_LOCALE` cookie, else the base
   locale), so the client settings arrive alongside the modules (1.1 s instead of 2.3 s
   on slow 4G).

Over plain HTTP/1.1 (local development) the browser still runs only 6 requests at a
time, so the time until the form is interactive barely changes there; behind an HTTP/2
reverse proxy all modules share one connection and the preloads take effect. SvelteKit
2.62 overrides custom chunk grouping in the client build, so merging the many small
chunks is not an option without fighting the framework.

**Switching back to master.** This branch also squashed the migrations into one
baseline (`20261110000000_initial_schema.sql`; see `server/CLAUDE.md`). On first boot it
adopts an existing database by rewriting its migration history to the baseline.
Master's server, which still has the 71 old migrations, then refuses that database.
To go back and forth, give master its own database (`DATABASE_URL`).

### What was built

| Piece | Where |
|---|---|
| Console session: table, entity, repository (Postgres + in-memory), use cases `start` / `resolve` / `end` | table `admin_web_sessions` (in the baseline migration), `domain/entities/admin_web_session.rs`, `application/use_cases/admin_web_session/` |
| Shared admin password check, used by `/admin/login` and the console sign-in | `application/shared/admin_credentials.rs` |
| `/session` handlers. `POST` sits in the credential router, under the strict bucket | `presentation/handlers/console_session_handler.rs` |
| Cookie, same-origin guard, `/api` bridge | `presentation/middleware/console_session.rs` |
| Static UI: SPA allow-list, CSP with script hashes, cache headers | `presentation/ui.rs`, `UI_DIR` in `config.rs` |
| Dashboard as a static SPA under `/console` | `dashboard/` (`adapter-static`, no `*.server.ts`, `scripts/assert-static.mjs`) |

### Where it departs from §6

1. **The session holds no token.**
   - §6.4 proposed a "token vault": seal the admin access and refresh tokens in the session row, and refresh them under a row lock.
   - The implementation stores only the hash of the cookie value. On each `/api` request, the bridge:
     - resolves the session;
     - re-reads the admin, which must still be active;
     - mints a fresh admin access token (HS256, 15 min) for that one request.
   - `require_admin` then verifies the token exactly as before. It stays untouched.
   - Consequences:
     - there is no refresh token and no refresh race, so there is nothing for reuse detection to trip;
     - there is no secret at rest;
     - deactivating an admin ends their console sessions on the next request.
2. **The bridge is a nested router, not a URI rewrite.**
   - `/api` nests the **admin router itself**, behind the bridge middleware.
   - Only admin paths exist under `/api`: `/api/me` and `/api/v1/...` are 404 by construction.
3. **The console moved to `/console`** (§8.1, the recommended option). `/login`, `/auth/*`, `/terms` and `/privacy` stay at the root. Old `/dashboard/*` links redirect there.
4. **Cookie flavour depends on the scheme.**
   - Over HTTPS the cookie is `__Host-gk_admin; Secure`.
   - Over plain-http development it is `gk_admin`, because the prefix requires `Secure`.
   - It is always `HttpOnly; SameSite=Strict`. The idle timeout is 2 h and the absolute lifetime 12 h.
5. **The same-origin guard** refuses:
   - any `Sec-Fetch-Site` other than `same-origin`;
   - any state-changing request whose `Origin` is not the origin of `AUTH_UI_BASE_URL`.

### Found along the way

- **zod and the CSP.** zod 4 probes for `eval` with `new Function`. The strict CSP had no `'unsafe-eval'`, before the port as well, so each probe raised a reported violation, although zod caught the error and nothing broke. `src/hooks.client.ts` sets `z.config({ jitless: true })`, which stops the probe.
- **Unknown paths now get a JSON 404.** Before, they got an empty 404. This also applies to API-only deployments. The fallback sits inside the global rate limiter.
- **`/console/tenants` has no page.** `/tenants` never had one either; the tenant list lives on the home page.

### Verified

- `cargo test`: 802 passed. That includes 23 new tests: the use cases, the cookie/origin guard, and UI serving (allow-list, headers, cache, path traversal, a rebuild picked up without a restart).
- `cargo test --features db-tests` (the new repository test): passed.
- `cargo sqlx prepare -- --lib`: 5 new entries, nothing removed.
- Dashboard: `npm run check` gave 0 errors. Its 73 warnings are all of the type already present (`state_referenced_locally`). `npm run build` writes a 20 MB static site.
- **HTTP end to end** against a running server: 39/39 checks pass. They cover:
  - the SPA served only on allow-listed paths, with the API untouched;
  - the CSP hash, framing, caching, brotli and path traversal;
  - `/session` and every bridge refusal (cross-site, same-site, foreign or missing `Origin`, forged cookie, bearer token on `/api`, cookie on bearer routes, the old cookie after sign-out);
  - the rate limit.
- **Headless Chromium**, with no console errors, no CSP violations and no cross-origin requests:
  - the signed-out redirect to `/login?next=…`;
  - sign-in through the form;
  - deep-link reloads;
  - the legacy redirect;
  - sign-out;
  - the hosted end-user sign-in, through a forced password change, to the app's callback with `code` and `state`;
  - the appearance editor's preview iframe.

### Not done on the branch

- **Packaging:** no Node build stage in the Dockerfile (§6.7). The image still ships the API only.
- **Configuration outside the repository:**
  - SDK and example-app configs still point `authUiBaseUrl` at the old dashboard origin;
  - deployment variables (`CORS_ALLOWED_ORIGINS`, `TRUSTED_PROXIES`) still list it.
- **Passkeys:** the WebAuthn RP ID migration (§8.2) is a deployment decision, not code.
- **CSP:** `style-src` keeps `'unsafe-inline'`, as before.


## 1. Verdict

**Yes, it is possible, and it can be done without losing any security control.** Several controls get stronger (§5).

You cannot simply point Axum at today's `dashboard/build/`. That build is a **Node application**, not a folder of files (`@sveltejs/adapter-node`, `svelte.config.js`). The Node half of the dashboard is itself a security component: a backend-for-frontend (BFF) that keeps the admin tokens out of the browser. It also holds the server-side admin guard and the HTML security headers. Rust cannot run that code, so the port has three steps:

1. **Build the dashboard as a static SPA** (`adapter-static`, `ssr = false`, one fallback `index.html`).
2. **Re-implement the Node server half in Rust.** It is five things:
   - the admin session store;
   - `/session`;
   - the `/api/*` bridge that turns the cookie into a bearer token;
   - the security headers on HTML;
   - a few redirects.
3. **Serve the static files from Axum**, with an explicit allow-list of SPA routes.

Two decisions are needed before any work starts (§8):

- **The URL layout of the admin console.** Its pages collide with the API's `/tenants/*` routes.
- **The host the hosted login UI lives on.** It decides the WebAuthn RP ID, and passkeys are bound to that ID.

---

## 2. What the dashboard is today

There are two applications in one SvelteKit project, on one origin.

| Part | Routes | Who uses it | How it talks to the IdP |
|---|---|---|---|
| **Admin console** | `(dashboard)` group (`/`, `/tenants/...`, `/organization`, `/account`), `/login` | Platform admins | Only through its own server: `/api/*` → `IDP_BASE_URL`, with a bearer token added server-side |
| **Hosted end-user login** | `/auth/[clientId]/*` (sign-in, sign-up, MFA, magic link, password flows), `/terms`, `/privacy` | Every relying party's users. The SDKs build `${authUiBaseUrl}/auth/${clientId}/sign-in` (`packages/gatekeeper-spa-oauth/src/oauth.ts:41`, `packages/gatekeeper-bff-core/src/oauth.ts:61`) | Directly from the browser, cross-origin (`PUBLIC_API_BASE_URL`), so the risk engine and the per-IP rate limits see the real client IP |

### Code that runs only on the Node server

All of this must disappear or move to Rust.

| File | What it does | Security role |
|---|---|---|
| `src/lib/server/session.ts` | In-process `Map` from session id to `{access, refresh, admin identity}`. HMAC-signed `gk_session` cookie: `HttpOnly`, `Secure`, `SameSite=Lax`, 12 h idle timeout | **Admin tokens never reach JavaScript.** An XSS cannot exfiltrate a credential that renews itself |
| `src/routes/session/+server.ts` | `POST` calls `/admin/login`, stores the token pair, sets the cookie and returns only the identity. `DELETE` calls `/admin/logout` (revokes the refresh family) and clears the cookie | Sign-in and sign-out without ever exposing a token |
| `src/routes/api/[...path]/+server.ts` and `src/lib/server/idp.ts` | Same-origin proxy: adds the bearer token and `X-Forwarded-For`, and refreshes once on a 401. Refresh is **single-flight**, so parallel 401s cannot trip refresh-token reuse detection | Keeps the token server-side; keeps the admin's real IP in audit and rate limits |
| `src/routes/(dashboard)/+layout.server.ts` | Redirects to `/login?next=<path-only>` unless `locals.admin` is set | Nothing of the console renders without a session |
| `src/hooks.server.ts` | Resolves the session. Sets CSP, `X-Frame-Options` (`SAMEORIGIN` only for `?preview=1`), `nosniff`, `Referrer-Policy` and HSTS on HTML responses. Does the paraglide locale substitution during SSR | Clickjacking defence for both the login page and the console; limits what an XSS can do |
| `src/routes/auth/[clientId]/+layout.server.ts`, `sign-up/+page.server.ts`, `forgot-password/+page.server.ts` | Fetches the client's auth data during SSR, redirects away from disabled pages, sets the locale cookie | **UX only.** The IdP enforces both rules itself: `can_self_register` in `register/use_case.rs:82` and `can_self_forgot_password` in `forgot_password/use_case.rs:75` |
| `src/routes/dashboard/**`, `src/routes/docs/**` (`+page.server.ts`) | 308 redirects for legacy URLs | None |

Everything else is already static: about 1,450 files, 20 MB in `build/client` including the `.br` and `.gz` variants.

---

## 3. The parts of the Rust server that matter here

### Router and layers

`presentation/routes.rs` merges the admin, `me`, credential and public routers, then applies these layers:

- a global per-IP governor: a burst of 200, then one request per 100 ms;
- `security_headers`, which **overwrites** these on every response: `CSP: default-src 'none'; frame-ancestors 'none'`, `X-Frame-Options: DENY`, `nosniff`, `no-referrer`, HSTS;
- metrics;
- client context (IP and User-Agent);
- request id.

`main.rs` then adds tracing, a 30 s timeout, gzip compression and CORS.

### CORS, authentication and admin tokens

- **CORS:** `build_cors` uses the allow-list in `CORS_ALLOWED_ORIGINS`, and allows any origin when that is empty. It never sets `allow_credentials`. Allowed methods are GET, POST, PUT and DELETE.
- **Authentication:** `require_admin` and `require_auth` read **only** `Authorization: Bearer` (`state.auth.authenticate(headers)`). The server uses no cookies anywhere today.
- **Admin tokens:**
  - `/admin/login` and `/admin/refresh` sit in the credential router, with the strict bucket; `/admin/logout` is public.
  - The data lives in the `admin_users` and `admin_refresh_tokens` tables.
  - Access tokens are HS256 and live 15 minutes; the refresh token rotates on every use.

### Login UI settings and OAuth endpoints

- **`AUTH_UI_BASE_URL`** (default `http://localhost:5173` before this change; now `http://localhost:8080`, the server's own origin) is used in two places:
  - the password-reset and magic-link emails;
  - the default **WebAuthn RP origin and RP ID** (`resolve_webauthn`, `config.rs:348`).
- **`/oauth/authorize`** returns JSON. It only validates the request; the SPA renders the screen.

### Path space

The API owns these paths:

`/admin/*`, `/me*`, `/tenants*`, `/v1/*`, `/oauth/*`, `/.well-known/*`, `/authentication/*`, `/health`, `/ready`

**Collision:** the console pages live at `/tenants/[tenantId]/...`. Those are the same paths as admin API GET routes such as `/tenants/{tenant_id}` and `/tenants/{tenant_id}/users`. A browser reload on `/tenants/abc/users` would reach the JSON API, not the SPA.

These paths are free for the UI:

`/`, `/login`, `/auth/*`, `/terms*`, `/privacy*`, `/session`, `/api/*`, `/_app/*`, `/robots.txt`, `/organization`, `/account`, `/dashboard/*`, `/docs/*`

---

## 4. Options considered

| Option | How it works | Verdict |
|---|---|---|
| **A. Axum serves a static SPA** | `adapter-static`, and Rust re-implements the Node server half | **Recommended.** One process, one origin, no Node in production |
| B. Axum reverse-proxies to Node | Rust forwards non-API paths to `node build` | Possible, but Rust isn't serving the files. It keeps two runtimes and two deploys, and adds a proxy hop. Only useful as a stepping stone |
| C. Server-side rendering in Rust | Render Svelte inside Rust | **Not viable.** It needs an embedded JavaScript runtime (V8 / deno_core): a large attack surface, and it breaks the crate's `#![forbid(unsafe_code)]` posture |

The rest of this document describes option A.

---

## 5. Security control by control

| # | Control | Today (Node) | After the port (Rust) | Result |
|---|---|---|---|---|
| 1 | Admin tokens unreadable by JavaScript | In-process `Map` plus an `HttpOnly` cookie | A Rust session store plus an `HttpOnly` `__Host-` cookie. Tokens never leave the server | **Kept** |
| 2 | Session storage | In-process `Map`. A restart signs everyone out, and more than one instance needs sticky sessions | A Postgres table. The cookie id is stored only as a SHA-256 hash, and the tokens are sealed with the existing AES-GCM `SecretCipher` | **Stronger:** survives restarts, works on every replica, can be revoked |
| 3 | Single-flight refresh | One in-flight promise per session, per process | `SELECT … FOR UPDATE` on the session row | **Stronger:** holds across replicas, so reuse detection cannot be tripped by two instances |
| 4 | CSRF on cookie-authenticated calls | `SameSite=Lax`, plus SvelteKit's origin check (forms and `text/plain`), plus JSON bodies only | `SameSite=Strict`, an explicit `Origin` / `Sec-Fetch-Site` check, and JSON only (`AppJson`) | **Stronger.** `Strict` costs nothing in an SPA, because the shell is public and authentication happens through same-origin `fetch` |
| 5 | Console guard | A server-side load redirects before anything renders | The shell is public static code. The guard is a client-side redirect when `GET /session` returns 401, and **all data still sits behind `require_admin`** | **Equivalent** (see note A). An optional Rust check on console HTML restores "nothing renders" |
| 6 | Admin IP attribution | Node forwards `X-Forwarded-For`, so the dashboard host must be in `TRUSTED_PROXIES` | Requests arrive directly: the TCP peer, or the real TLS proxy | **Stronger:** one fewer trusted forwarder, so less room to spoof `X-Forwarded-For` |
| 7 | Hosted login calls the IdP directly, so the risk engine sees the real IP | Browser to IdP, cross-origin | Browser to IdP, same origin | **Kept and simpler:** the login UI no longer needs CORS |
| 8 | CSP and clickjacking on HTML | `hooks.server.ts` | A Rust layer for HTML responses, with the `?preview=1` exception | **Kept, and can be tightened:** `connect-src 'self'` only, and script hashes instead of `'unsafe-inline'` (§6.5) |
| 9 | Hardening headers on API JSON | `default-src 'none'; frame-ancestors 'none'` | Unchanged for API routes | **Kept** |
| 10 | Rate limit on the admin password | `/session` calls `/admin/login`, which is in the strict credential bucket | `POST /session` must be registered in the **credential router** | **Kept, if placed correctly** (a trap; see §6.4) |
| 11 | Sign-out revokes the refresh family | Best-effort `/admin/logout` | An in-process call to the same use case, then delete the row | **Kept** |
| 12 | Email-template preview isolation | `srcdoc` in `sandbox=""` (opaque origin). The API returns JSON | Unchanged | **Kept** |

### Note A: the console guard

The console's JavaScript holds no secrets and no data; `$env/dynamic/private` disappears completely. The warning in `(dashboard)/+layout.server.ts` reads: "any `+page.server.ts` added under this group would run with no auth check". That class of bug **cannot exist** without server code.

Add a CI check that fails the dashboard build if any of these files appear:

- `+page.server.ts`
- `+layout.server.ts`
- `+server.ts`
- `hooks.server.ts`

### The one new risk: API and UI on one origin

**What changes.** Today the console and the hosted login already share one origin (the dashboard host), and the API is on another. After the port, all three share the issuer origin. An XSS anywhere in the UI then runs on the same origin as the API.

**Why the impact stays low:**

- **The cookie authenticates only the `/api` bridge**, and the bridge allows only the admin surface. An XSS in today's console can already drive `/api` as the admin, so nothing new is exposed.
- API responses are JSON with `nosniff` and `default-src 'none'`.
- **No API endpoint returns HTML.** Verified: the email preview returns JSON, which the dashboard renders in a `sandbox=""` iframe.

**Invariants to keep forever.** Review checklist and tests:

- No API endpoint ever returns user-controlled content as `text/html`, or as any other type a browser executes.
- The session cookie is never accepted outside `/api/*` and `/session`.
- CORS never sets `allow_credentials(true)`.

**This is the industry norm.** Keycloak, Auth0 Universal Login and Okta all host the login UI on the issuer origin. It also helps against phishing: users only need to trust one domain, and the WebAuthn RP ID becomes the issuer's domain.

---

## 6. Recommended design

### 6.1 Dashboard: static build

- **Adapter:** `svelte.config.js` uses `adapter-static({ pages: 'build', assets: 'build', fallback: 'index.html', precompress: true, strict: true })`.
- **Rendering:** a root `src/routes/+layout.ts` with `export const ssr = false;`. The whole app renders on the client; `/terms` and `/privacy` could be prerendered later.
- **Delete:**
  - `src/hooks.server.ts`, `src/lib/server/*`;
  - `src/routes/api/`, `src/routes/session/`;
  - every `+page.server.ts` and `+layout.server.ts`.
- **Convert:**
  - `auth/[clientId]/+layout.server.ts` becomes `+layout.ts`, with the same logic. `redirect()` thrown from a universal load works on the client.
  - The sign-up and forgot-password guards become `+page.ts`.
- **Console guard:** `(dashboard)/+layout.ts` calls `GET /session`. It gets back `{ admin }`, or redirects to `/login?next=…` using the same path-only logic, which cannot become an open redirect.
- **Base URLs:**
  - With one origin, API calls become **relative**, so `PUBLIC_API_BASE_URL` goes away.
  - `PUBLIC_WEBSITE_BASE_URL` moves to `$env/static/public`. In a static build, `$env/dynamic/public` is frozen at build time anyway.
  - `providers-tab.svelte` displays the Google/GitHub callback URL from `PUBLIC_API_BASE_URL`. Switch it to `location.origin`.
- **Locale:** nothing replaces `%paraglide.lang%` / `%paraglide.dir%` in `app.html` without a server.
  - Hard-code a default (`lang="en"`), then set `document.documentElement.lang` / `dir` from `getLocale()` in the root layout.
  - Paraglide's default strategy (cookie, then global variable, then base locale) works client-side.
  - The auth layout's "adopt the client's default language" cookie becomes a call to `setLocale`.
- **Legacy redirects:** `/dashboard/*` and `/docs/*` move to Rust as 308 handlers, or become client-side redirects.
- **Links:** there are **150 hard-coded `/tenants…` links in 59 files** that need the console prefix (§6.2). The change is mechanical. Do it with `resolve()` from `$app/paths`, so any future move is free.
- **Dev workflow:** in `vite dev`, proxy `/api`, `/session`, `/v1`, `/oauth`, `/authentication` and `/.well-known` to `:8080`. Development stays same-origin, like production.

### 6.2 URL layout (fixing the collision)

Recommended layout:

```
/                        console home (or a redirect to /console)
/console/...             admin console   (route folder `(dashboard)` becomes `console/`)
/console/login           admin sign-in
/auth/{clientId}/...     hosted login    (same path; the SDK URL shape keeps working)
/terms, /privacy[/{id}]  legal pages
/session                 admin session endpoint (Rust)
/api/{admin path}        cookie-to-bearer bridge (Rust)
/_app/*, /robots.txt     static assets
everything else          the API, exactly as today
```

Alternatives:

- **`kit.paths.base = '/ui'`.** The whole app lives under `/ui`, with `AUTH_UI_BASE_URL=https://id.example.com/ui`. The SDKs concatenate the base URL, so they keep working. But `resolve_webauthn` derives the WebAuthn origin from `AUTH_UI_BASE_URL`, path included, so `WEBAUTHN_RP_ORIGIN` must then be set explicitly. Workable, but `/console` is cleaner.
- **Content negotiation on `/tenants/*`.** Serve the SPA when `Sec-Fetch-Dest: document` or `Accept: text/html`. It leaks nothing, because the shell is public. **Not recommended:** it is brittle with caches (it needs `Vary`) and with older clients, and it blurs the line between an API 404 and a page.

### 6.3 Static serving in Axum

- **Where the files come from.** Use the tower-http `fs` feature (`ServeDir` / `ServeFile`) and read the files from **`UI_DIR` at runtime**. When `UI_DIR` is unset, no UI is mounted, so API-only deployments are unchanged.
  - An optional `embedded-ui` cargo feature with `rust-embed` would give a single binary.
  - CI runs `cargo … --all-features`, so the embed must tolerate a missing folder (`#[allow_missing = true]`), or CI has to build the UI first.
- **Rate limit and headers.** Mount the asset routes **outside the `GovernorLayer`.** A cold page load fetches dozens of chunks. Under the global per-IP bucket, admins behind one NAT would starve each other's API calls. Also mount them outside the API `security_headers` layer, with their own headers.
- **Fallback.** Serve the SPA fallback **only for allow-listed prefixes.** Every other path falls through to the API and gets a JSON 404, never `index.html`.
- **Cache:**
  - `/_app/immutable/*`: `public, max-age=31536000, immutable`;
  - `index.html`, `/_app/version.json` and `/_app/env.js`: `no-cache`.
- **Compression:** serve the precompressed files with `ServeDir::…precompressed_br().precompressed_gzip()`. `adapter-node` already emits `.br` / `.gz`, and `adapter-static` does the same with `precompress: true`.
- **Path traversal:** `ServeDir` already rejects `..`. The `rust-embed` variant has no filesystem access at all.

Sketch (illustrative, not final code):

```rust
// presentation/ui.rs (new)
use std::path::Path;
use axum::{Router, middleware};
use tower_http::services::{ServeDir, ServeFile};

pub fn ui_router(ui_dir: &Path) -> Router {
    let index = ServeFile::new(ui_dir.join("index.html"));
    let assets = ServeDir::new(ui_dir.join("_app"))
        .precompressed_br()
        .precompressed_gzip();

    Router::new()
        .nest_service("/_app", assets) // + a layer adding the immutable Cache-Control
        .route_service("/robots.txt", ServeFile::new(ui_dir.join("robots.txt")))
        // SPA entry points: an explicit allow-list, never a catch-all.
        .route_service("/", index.clone())
        .route_service("/console", index.clone())
        .route_service("/console/{*rest}", index.clone())
        .route_service("/auth/{*rest}", index.clone())
        .route_service("/terms", index.clone())
        .route_service("/terms/{client_id}", index.clone())
        .route_service("/privacy", index.clone())
        .route_service("/privacy/{client_id}", index)
        .layer(middleware::from_fn(html_security_headers))
}
```

In `routes::router`, merge the UI router next to the API routers, and apply the governor and `security_headers` only to the API part. One rule matters: `security_headers` currently **overwrites** CSP on every response. It must not touch UI responses, either by splitting the routers or by not overwriting a CSP that is already set.

### 6.4 Admin session in Rust (replaces `session.ts`, `/session` and the `/api` proxy)

> **Implemented differently**: the session holds no token at all; see §0, "Where it departs from §6".

**Recommended variant: a "token vault".** It behaves exactly like today's proxy, and it leaves `require_admin` and the admin token flow untouched.

**Table.** A new migration:

```sql
CREATE TABLE admin_web_sessions (
    id_hash              BYTEA PRIMARY KEY,        -- SHA-256 of the cookie's random id; the raw id is never stored
    admin_user_id        UUID NOT NULL REFERENCES admin_users(id) ON DELETE CASCADE,
    access_token_sealed  BYTEA NOT NULL,           -- SecretCipher (AES-256-GCM), like other secrets at rest
    refresh_token_sealed BYTEA NOT NULL,
    access_expires_at    TIMESTAMPTZ NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at         TIMESTAMPTZ NOT NULL DEFAULT now(),   -- idle timeout (12 h today)
    expires_at           TIMESTAMPTZ NOT NULL                  -- NEW absolute cap; today there is only the idle timeout
);
```

The existing retention purge deletes expired rows.

**Cookie.** `__Host-gk_admin=<32 random bytes, base64url>` with `HttpOnly; Secure; SameSite=Strict; Path=/` and no `Domain`. The `__Host-` prefix makes the browser enforce `Secure`, no `Domain`, and `Path=/`. Today's HMAC on the cookie becomes optional, because lookup is by the hash of a 256-bit random id. Current Chrome and Firefox accept it on `http://localhost`; Safari may need HTTPS in development.

**Endpoints:**

- **`POST /session`**, in the **credential router**, so the strict per-IP bucket applies as it does to `/admin/login` today.
  - Takes `{email, password}` and calls the admin login use case in-process: same audit, same generic 401, same 429.
  - (Since then: `mfaCode` too, for an admin with two-step verification. Both paths share `AdminAuthenticator`; see `FLOWS-AUTHENTICATION.md` §5.)
  - Stores the row, sets the cookie, and returns `{admin}` with `Cache-Control: no-store`.
- **`GET /session`** returns `{admin}`, or 401. It serves the client-side guard and the sidebar identity.
- **`DELETE /session`** revokes the refresh family through the admin logout use case, deletes the row and clears the cookie.

**Bridge middleware.** It wraps the whole `Router` as a service, because a URI rewrite must happen before routing (axum: "Rewriting request URI in middleware"). It runs these steps in order:

1. **Act only on an allow-list:** `/api/admin/*` and `/api/tenants*`. Every other `/api/*` path, such as `/api/me` or `/api/v1/...`, gets a 404. The cookie must never become a general credential.
2. **Refuse cross-site requests.**
   - `Sec-Fetch-Site`, when present, must be `same-origin`.
   - For non-GET/HEAD methods, `Origin` must equal the server's configured public origin.
3. **Remove any `Authorization` header the client sent.**
4. **Resolve the cookie to a session row.** If the access token expires within about 30 s, refresh in-process through the admin refresh use case, **under a row lock** (single-flight across replicas). Then re-seal the tokens and update the row.
5. **Insert `Authorization: Bearer <access>`**, strip `/api` from the URI, and touch `last_seen_at` (throttled).
6. **Set `Cache-Control: no-store` on the response**, with no CORS headers.

```rust
// Sketch.
async fn admin_cookie_bridge(
    State(s): State<BridgeState>,
    mut req: Request,
    next: Next,
) -> Response {
    let Some(rest) = req.uri().path().strip_prefix("/api").map(str::to_owned) else {
        return next.run(req).await;
    };
    if !(rest.starts_with("/admin/") || rest.starts_with("/tenants")) {
        return not_found();
    }
    if !is_same_origin(&req, &s.public_origin) {
        return forbidden("cross-origin request refused");
    }
    req.headers_mut().remove(header::AUTHORIZATION);

    let Some(session) = s.sessions.resolve(session_cookie(&req)).await else {
        return unauthorized();
    };
    let Ok(access) = s.sessions.fresh_access_token(&session).await else {
        return unauthorized();
    };
    req.headers_mut().insert(header::AUTHORIZATION, bearer(&access));
    *req.uri_mut() = with_path(req.uri(), &rest);

    let mut res = next.run(req).await;
    res.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

// main.rs: wrap the router so the rewrite happens before routing.
// let app = middleware::from_fn_with_state(bridge_state, admin_cookie_bridge).layer(router);
// axum::serve(listener, ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(app))
```

After the rewrite, bridged calls go through the same layers as a direct API call today (`capture_client_context`, governor, `require_admin`, audit), so the per-IP limits and the audit attribution stay the same.

**Simpler alternative: a "native cookie principal".** The session row would produce the `AuthPrincipal` (platform admin) directly, with no JWTs. There are fewer moving parts, but it adds a second authentication path inside `require_admin`, the most security-sensitive middleware. Consider it later, once the port is stable.

### 6.5 Security headers on HTML (Rust)

For UI documents:

```
Content-Security-Policy: default-src 'self'; script-src 'self' 'sha256-<bootstrap>'; style-src 'self' 'unsafe-inline';
                         img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; object-src 'none';
                         base-uri 'self'; frame-ancestors 'none'; form-action 'self'
X-Frame-Options: DENY      (SAMEORIGIN, and frame-ancestors 'self', only for /auth/* with ?preview=1)
X-Content-Type-Options: nosniff
Referrer-Policy: no-referrer
Strict-Transport-Security: max-age=63072000; includeSubDomains
Cache-Control: no-cache
```

- **`connect-src 'self'`** is tighter than today, which also has to allow the IdP's origin.
- **Script hash.** The fallback `index.html` has one inline bootstrap script. Rust can compute its SHA-256 at startup, or a build step can write it. That drops `'unsafe-inline'` from `script-src`, which today's `hooks.server.ts` leaves as follow-up work. A first iteration may keep `'unsafe-inline'` and still match today.
  - `style-src 'unsafe-inline'` stays, because Svelte and bits-ui apply inline styles at runtime.
- **Preview exception.** Scope the `?preview=1` framing exception to `/auth/*` only. Today it applies to any HTML path with `?preview=1`, so this is a small tightening.
- **Already true today, unchanged by the port:** `img-src 'self' data: blob:`, although login appearance lets admins set http(s) logo and background URLs. Decide separately whether to allow-list image hosts.
- **API routes** keep `default-src 'none'; frame-ancestors 'none'`.

### 6.6 Configuration changes

| Variable | Today | After the port |
|---|---|---|
| `AUTH_UI_BASE_URL` | Dashboard origin | The server's own public origin (for example `https://id.example.com`) |
| `WEBAUTHN_RP_ID` / `WEBAUTHN_RP_ORIGIN` | Derived from `AUTH_UI_BASE_URL` | The issuer host. **See §8.2: passkeys are bound to the RP ID** |
| `CORS_ALLOWED_ORIGINS` | Must include the dashboard origin | Drop the dashboard origin; keep only third-party SPAs that call the API |
| `TRUSTED_PROXIES` | The dashboard host and the TLS proxy | The TLS proxy only |
| `UI_DIR` (new) | — | Path to the static build. Unset means the UI is disabled |
| Public origin for the CSRF check (new, or derived from the issuer) | — | The exact value `Origin` must match |
| Dashboard `SESSION_SECRET`, `IDP_BASE_URL`, `PUBLIC_API_BASE_URL` | Required | Removed |

### 6.7 Packaging: where the code lives

- **Keep the source in `dashboard/`.** The repository is a monorepo, and this keeps the Node toolchain out of the crate. "Porting into `/server`" should mean shipping the **build output** with the server, not moving the source. Moving the source under `server/ui/` is possible, but it mixes two toolchains in the crate directory and gains nothing.
- **Dockerfile.** Add a `node:22-bookworm-slim` stage that runs `npm ci && npm run build` in `dashboard/`. Copy `build/` into the runtime image at `/app/ui`, and set `UI_DIR=/app/ui`. The build context is `server/` today, so either:
  - move the context to the repository root; or
  - pass the UI as a second context (`docker build --build-context ui=../dashboard …`).
- **CI.** Rust CI is unaffected when the UI comes from `UI_DIR` at runtime. Dashboard CI (`npm run check`, `npm run build`) stays separate, plus the "no server files" check from note A.

---

## 7. Work list

### Dashboard

1. Switch to `adapter-static` with a fallback, add `ssr = false`, and delete the server code listed in §2.
2. Convert the auth layout and page guards to universal loads.
3. Add the console guard through `GET /session`.
4. Move `(dashboard)` to `console/` and prefix the 150 links using `resolve()`.
5. Switch to relative API URLs and `$env/static/public`. Fix the callback URL in `providers-tab.svelte`.
6. Handle `lang` / `dir` on the client; move the locale cookie to `setLocale`.
7. Set up the Vite dev proxy.
8. Add the CI check against server files.

### Server

1. Add the `admin_web_sessions` migration, its repository (DB and in-memory) and the use cases `create` / `resolve` / `refresh` / `destroy`, with tests.
2. Add the `/session` handlers. Put `POST` in the credential router.
3. Add the bridge middleware: allow-list, same-origin check, Authorization stripping, refresh under row lock, URI rewrite. Wrap it around the router in `main.rs`.
4. Add `presentation/ui.rs`: static serving, the SPA allow-list, cache headers, and HTML security headers (with the optional script hash).
5. Keep `security_headers` from overwriting UI headers, and keep the governor off static assets.
6. Add the 308 handlers for `/dashboard/*` and `/docs/*`.
7. Config: `UI_DIR`, the public origin, and startup validation (in production, refuse to start if `UI_DIR` is set but `index.html` is missing). Enable the tower-http `fs` feature.
8. Update the docs: `server/CLAUDE.md` environment list, `FLOWS-*`, and the deploy notes.

### Operations

1. Add the Dockerfile Node stage; update compose (one service instead of two).
2. Update `AUTH_UI_BASE_URL`, `CORS_ALLOWED_ORIGINS`, `TRUSTED_PROXIES` and the WebAuthn variables.
3. Update the SDK / relying-party `authUiBaseUrl` settings.
4. Make the old dashboard host permanently redirect (301/308) to the new one for at least the reset and magic-link token lifetime.
5. Decommission the Node service.

---

## 8. Decisions and risks

1. **The console URL prefix.** `/console/` (recommended) or `kit.paths.base`. See §6.2.
2. **The login UI host and the WebAuthn RP ID. This is the biggest migration risk.** Passkeys are cryptographically bound to the RP ID. Today that defaults to the host of `AUTH_UI_BASE_URL`, the dashboard host. If users in production already have passkeys registered under it, serving the login UI from a different host **makes those passkeys unusable** (both MFA and passwordless login). Options:
   - **(a)** Not in production yet: switch now, at no cost.
   - **(b)** The RP ID is already a registrable parent domain (`example.com`) and both hosts are its subdomains: keep the RP ID and add the new origin through `WEBAUTHN_ADDITIONAL_ORIGINS`.
   - **(c)** Keep the old hostname: point it at the Rust server, which then serves the UI under that name. The issuer URL is unaffected, because the SPA calls same-origin relative URLs on whichever host it was loaded from. This needs the Rust server to answer on both hostnames, and the public origin for the CSRF check to be that host.
   - **(d)** WebAuthn Related Origin Requests (`/.well-known/webauthn`): browser support is still partial, so use it only as an addition.

   Separately:
   - every relying party's `authUiBaseUrl` changes;
   - reset and magic-link emails already sent point at the old host, hence the temporary redirect in §7.
3. **No more SSR.** The hosted login shows a short loading state while `auth-data` loads: one same-origin request. There is no security impact. If it matters, Rust can later inline that JSON into the HTML.
4. **API and UI on one origin.** Accepted, with the invariants in §5.
5. **Session store.** Use Postgres. The server is designed to run several replicas (the dispatcher and the retention purge already say so), and an in-memory store would need sticky sessions.

---

## 9. Migration plan

Each step can ship on its own.

1. **Decide** §8.1 and §8.2.
2. **Rust session and bridge.** Add `admin_web_sessions`, `/session` and the bridge, behind the UI being disabled. They are unused but tested.
3. **Static dashboard build.** Do it on a branch: the static adapter, the conversions, `/console`, relative URLs. During this phase it can be served by any static server in front of the Rust API, to test.
4. **Rust static serving.** Add `presentation/ui.rs` and the HTML headers, enabled by `UI_DIR`.
5. **Staging end to end.** Cover:
   - **Sign-in:** password, email/TOTP/WebAuthn MFA, passwordless passkey, magic link, social login.
   - **Account flows:** password reset, sign-up and the legal pages.
   - **Console:** admin create/read/update/delete, the appearance preview iframe, and the email preview.
   - **Sign-out:** it revokes the refresh family.
   - **Headers:** zero CSP violations in the console log.
   - **Load:** a cold page load stays under the rate limits.
6. **Cutover.** Change DNS and environment variables, put the old-host redirect in place, and decommission Node.

## 10. Rough effort

| Area | Size | Time |
|---|---|---|
| Rust (migration, repository, use cases, `/session`, bridge, UI router, headers, config, tests) | About 700–1,000 lines | 3–5 days |
| Dashboard (mostly mechanical: prefix links, remove server code, convert loads) | — | 1–2 days |
| Packaging, environment, staging tests | — | 1–2 days |

## 11. Security tests to write

- **Session cookie:** the attributes are exactly `__Host-gk_admin; HttpOnly; Secure; SameSite=Strict; Path=/`, with no `Domain`.
- **Bridge, rejections:**
  - `/api/tenants` without a cookie returns 401;
  - with a forged or unknown cookie it returns 401;
  - `/api/me` and `/api/v1/...` return 404 even with a valid cookie.
- **Bridge, CSRF:**
  - a `POST /api/...` with `Origin: https://evil.example` returns 403;
  - `Sec-Fetch-Site: cross-site` (and `same-site`) returns 403.
- **Bridge, headers:**
  - an `Authorization` header sent by the client never reaches the handler;
  - bridged responses carry `Cache-Control: no-store`.
- **Sign-in and sign-out:**
  - the `POST /session` rate limit is the credential bucket; the 11th attempt in a burst gets 429, matching `/admin/login`;
  - after `DELETE /session`, the old refresh token is rejected (the family is revoked) and the row is gone.
- **Concurrent refresh:** two parallel bridged requests with an expired access token, on two app instances, do not trip refresh-token reuse detection.
- **HTML and fallback:**
  - HTML carries the UI CSP, and API JSON keeps `default-src 'none'`;
  - `?preview=1` relaxes framing only under `/auth/*`;
  - an unknown path, such as `/tenants/x/nope` or `/foo`, returns the API's JSON 404, never `index.html`.
- **Static files:**
  - `/_app/../../etc/passwd` and encoded variants return 404;
  - only `/_app/immutable/*` is cacheable as `immutable`.

---

## Appendix: files examined

- **Dashboard:**
  - `svelte.config.js`, `package.json`, `.env.example`, `src/app.html`
  - `src/hooks.server.ts`, `src/hooks.ts`
  - `src/lib/server/{session,idp,legacy-redirect}.ts`
  - `src/lib/services/base/{api,admin-api}.ts`
  - `src/routes/api/[...path]/+server.ts`, `src/routes/session/+server.ts`
  - every `+page.server.ts` / `+layout.server.ts` / `+page.ts`
  - the appearance and email preview iframes
  - `src/lib/types/login-appearance.ts`
- **Server:**
  - `src/main.rs`, `src/presentation/routes.rs` and `routes/{admin,me,credential,public}.rs`
  - `src/presentation/middleware/auth.rs` (`require_admin`, `require_auth`)
  - `src/infraestructure/config.rs` (`AUTH_UI_BASE_URL`, WebAuthn, CORS, `TRUSTED_PROXIES`)
  - `oauth_authorize_handler.rs`, `email_template_handler.rs`
  - the register and forgot-password use cases
  - `Dockerfile`, `docker-compose.prod.yml`, `.github/workflows/rust-ci.yml`
- **SDKs:** `packages/gatekeeper-spa-oauth/src/oauth.ts`, `packages/gatekeeper-bff-core/src/oauth.ts` (how they build the login URL)
