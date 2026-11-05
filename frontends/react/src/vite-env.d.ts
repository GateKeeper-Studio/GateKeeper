/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_GATEKEEPER_IDP_BASE_URL?: string;
  readonly VITE_GATEKEEPER_AUTH_UI_BASE_URL?: string;
  readonly VITE_GATEKEEPER_TENANT_ID?: string;
  readonly VITE_GATEKEEPER_CLIENT_ID?: string;
  readonly VITE_GATEKEEPER_REDIRECT_URI?: string;
  readonly VITE_GATEKEEPER_SCOPE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
