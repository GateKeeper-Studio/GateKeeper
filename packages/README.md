# GateKeeper client SDKs

The `@gatekeeper/*` packages connect an application to the GateKeeper identity
provider. There are two kinds of client, and each kind has one framework-agnostic core
with thin framework bindings on top. Put OAuth logic in the cores, not in the bindings.

## Public clients (SPAs)

The app runs only in the browser. It uses Authorization Code + PKCE, has **no client
secret**, and keeps tokens in `sessionStorage`. Register the OAuth client as `public` and
add the app's origin to the server's `CORS_ALLOWED_ORIGINS`.

| Package | For | Built on | Example |
|---|---|---|---|
| [`@gatekeeper/spa-oauth`](gatekeeper-spa-oauth/) | Any framework (core) | — | [`frontends/vanilla-js`](../frontends/vanilla-js/) uses the same flow |
| [`@gatekeeper/react`](gatekeeper-react/) | React 18 / 19 | `spa-oauth` | [`frontends/react`](../frontends/react/) |
| [`@gatekeeper/vue`](gatekeeper-vue/) | Vue 3 | `spa-oauth` | [`frontends/vue`](../frontends/vue/) |
| [`@gatekeeper/angular`](gatekeeper-angular/) | Angular 2+ | `spa-oauth` | [`frontends/angular`](../frontends/angular/) |
| [`@gatekeeper/angularjs`](gatekeeper-angularjs/) | AngularJS 1.x | `spa-oauth` | [`frontends/angularjs`](../frontends/angularjs/) |

## Confidential clients (BFFs)

A server you control (the backend-for-frontend) holds the client secret and the
tokens, and gives the browser only an encrypted httpOnly cookie. Register the OAuth
client as `confidential`. No CORS setup is needed. Add the BFF's address to the server's
`TRUSTED_PROXIES` if it forwards the browser IP.

| Package | For | Built on | Example |
|---|---|---|---|
| [`@gatekeeper/bff-core`](gatekeeper-bff-core/) | Any server framework (core) | — | — |
| [`@gatekeeper/nextjs`](gatekeeper-nextjs/) | Next.js App Router | `bff-core` | [`frontends/nextjs`](../frontends/nextjs/) |
| [`@gatekeeper/remix`](gatekeeper-remix/) | Remix | `bff-core` | [`frontends/remix`](../frontends/remix/) |
| [`@gatekeeper/sveltekit-bff`](gatekeeper-sveltekit-bff/) | SvelteKit (server helpers + `/svelte` components for profile and MFA) | its own copy of the BFF core | [`frontends/sveltekit-bff`](../frontends/sveltekit-bff/) |
| [`@gatekeeper/browser`](gatekeeper-browser/) | Browser side of any BFF app: profile client, MFA client, `<gatekeeper-mfa-configuration>` web component | — | — |

The BFF packages request the `account` scope by default, because their profile and MFA
helpers call the server's `/me` API. The OAuth client must have `account` in its
allowed scopes; otherwise the server drops it and `/me` calls fail with
`insufficient_scope`. Removing an MFA method requires the user's current password
(`removeMethod(id, currentPassword)`).

## Build

Every package builds with TypeScript:

```bash
cd packages/<package>
npm install
npm run build        # tsc -p tsconfig.json
```

Packages depend on each other through `file:` links (for example
`@gatekeeper/react` → `file:../gatekeeper-spa-oauth`), so build a core before the
bindings that use it. `@gatekeeper/angular` is consumed from source by its example app.

Each package has its own README with installation and usage.
