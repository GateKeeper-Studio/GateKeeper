import { GateKeeperMfaConfigurationElement } from './mfa-configuration.js';
import { GateKeeperProviderElement } from './provider.js';

export { GateKeeperProviderElement, getClosestGateKeeperClient } from './provider.js';
export { GateKeeperMfaConfigurationElement } from './mfa-configuration.js';

export function defineGateKeeperElements() {
  if (!customElements.get('gatekeeper-provider')) {
    customElements.define('gatekeeper-provider', GateKeeperProviderElement);
  }

  if (!customElements.get('gatekeeper-mfa-configuration')) {
    customElements.define('gatekeeper-mfa-configuration', GateKeeperMfaConfigurationElement);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'gatekeeper-provider': GateKeeperProviderElement;
    'gatekeeper-mfa-configuration': GateKeeperMfaConfigurationElement;
  }
}
