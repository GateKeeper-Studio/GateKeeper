/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue';
  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>;
  export default component;
}

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
