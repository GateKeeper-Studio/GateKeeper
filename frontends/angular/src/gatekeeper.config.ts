import type { GateKeeperSpaConfig } from '@gatekeeper/angular';

// Angular has no Vite-style import.meta.env, so edit these values directly (or wire
// them to your own environment.ts). Fill tenantId/clientId from the register script.
export const gatekeeperConfig: GateKeeperSpaConfig = {
  // The identity provider; it also serves the hosted login UI the app redirects to.
  idpBaseUrl: 'http://localhost:8080',
  tenantId: '',
  clientId: '',
  redirectUri: 'http://localhost:5177/',
  scope: 'openid profile email'
};
