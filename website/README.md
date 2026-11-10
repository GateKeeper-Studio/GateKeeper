# GateKeeper website

Standalone SvelteKit application for the public landing page and documentation.

## Development

```sh
cp .env.example .env
npm install
npm run dev
```

The development server uses `http://localhost:5174`. Its sign-in links go to the
dashboard, which the identity provider serves at `http://localhost:8080`
(`PUBLIC_DASHBOARD_BASE_URL`).

## Validation

```sh
npm run check
npm run build
```

Set `PUBLIC_DASHBOARD_BASE_URL` to the deployed dashboard origin before building for
production.
