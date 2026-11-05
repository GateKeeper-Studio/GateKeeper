import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { GateKeeperProvider, type GateKeeperSpaConfig } from '@gatekeeper/react';
import { App } from './App';
import './styles.css';

const env = import.meta.env;

const config: GateKeeperSpaConfig = {
  idpBaseUrl: env.VITE_GATEKEEPER_IDP_BASE_URL || 'http://localhost:8080',
  authUiBaseUrl: env.VITE_GATEKEEPER_AUTH_UI_BASE_URL || 'http://localhost:5173',
  tenantId: env.VITE_GATEKEEPER_TENANT_ID || '',
  clientId: env.VITE_GATEKEEPER_CLIENT_ID || '',
  redirectUri: env.VITE_GATEKEEPER_REDIRECT_URI || `${window.location.origin}/`,
  scope: env.VITE_GATEKEEPER_SCOPE || 'openid profile email'
};

createRoot(document.getElementById('root') as HTMLElement).render(
  <StrictMode>
    <GateKeeperProvider config={config}>
      <App />
    </GateKeeperProvider>
  </StrictMode>
);
