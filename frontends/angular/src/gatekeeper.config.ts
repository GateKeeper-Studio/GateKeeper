import type { GateKeeperSpaConfig } from '@gatekeeper/angular';

// Angular has no Vite-style import.meta.env, so edit these values directly (or wire
// them to your own environment.ts). Fill tenantId/clientId from the register script.
export const gatekeeperConfig: GateKeeperSpaConfig = {
  idpBaseUrl: 'http://localhost:8080',
  authUiBaseUrl: 'http://localhost:5173',
  tenantId: '',
  clientId: '',
  redirectUri: 'http://localhost:5177/',
  scope: 'openid profile email'
};
