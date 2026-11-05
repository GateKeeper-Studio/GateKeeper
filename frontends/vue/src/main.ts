import { createApp } from 'vue';
import { createGateKeeper } from '@gatekeeper/vue';
import App from './App.vue';
import './styles.css';

const env = import.meta.env;

createApp(App)
  .use(
    createGateKeeper({
      config: {
        idpBaseUrl: env.VITE_GATEKEEPER_IDP_BASE_URL || 'http://localhost:8080',
        authUiBaseUrl: env.VITE_GATEKEEPER_AUTH_UI_BASE_URL || 'http://localhost:5173',
        tenantId: env.VITE_GATEKEEPER_TENANT_ID || '',
        clientId: env.VITE_GATEKEEPER_CLIENT_ID || '',
        redirectUri: env.VITE_GATEKEEPER_REDIRECT_URI || `${window.location.origin}/`,
        scope: env.VITE_GATEKEEPER_SCOPE || 'openid profile email'
      }
    })
  )
  .mount('#app');
