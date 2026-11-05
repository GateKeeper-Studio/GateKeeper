import { createGateKeeperBrowserClient } from '../client.js';
import type {
  BrowserSafeSession,
  GateKeeperBrowserClient,
  GateKeeperBrowserClientOptions,
  GateKeeperBrowserSnapshot
} from '../types.js';

function parseBooleanAttribute(value: string | null, defaultValue: boolean): boolean {
  if (value === null) return defaultValue;
  return !['false', '0', 'off', 'no'].includes(value.toLowerCase());
}

function parseNumberAttribute(value: string | null, defaultValue: number): number {
  if (!value) return defaultValue;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : defaultValue;
}

function parseSessionAttribute(value: string | null): BrowserSafeSession | null {
  if (!value) return null;
  try {
    return JSON.parse(value) as BrowserSafeSession;
  } catch {
    return null;
  }
}

export class GateKeeperProviderElement extends HTMLElement {
  static observedAttributes = [
    'session-endpoint',
    'profile-endpoint',
    'refresh-skew-seconds',
    'min-refresh-delay-seconds',
    'auto-refresh',
    'initial-session'
  ];

  #client: GateKeeperBrowserClient | null = null;
  #unsubscribe: (() => void) | null = null;
  #initialSession: BrowserSafeSession | null | undefined;

  get client(): GateKeeperBrowserClient {
    if (!this.#client) this.#connectClient();
    return this.#client as GateKeeperBrowserClient;
  }

  set client(client: GateKeeperBrowserClient) {
    this.#disconnectClient();
    this.#client = client;
    this.#unsubscribe = client.subscribe((snapshot) => this.#publishSnapshot(snapshot));
  }

  get initialSession(): BrowserSafeSession | null | undefined {
    return this.#initialSession;
  }

  set initialSession(session: BrowserSafeSession | null | undefined) {
    this.#initialSession = session;
    this.#client?.setSession(session ?? null);
  }

  connectedCallback() {
    if (!this.#client) this.#connectClient();
  }

  disconnectedCallback() {
    this.#disconnectClient();
  }

  attributeChangedCallback() {
    if (!this.isConnected || !this.#client) return;
    this.#connectClient();
  }

  getSnapshot(): GateKeeperBrowserSnapshot {
    return this.client.getSnapshot();
  }

  refresh(): Promise<BrowserSafeSession | null> {
    return this.client.refresh();
  }

  #connectClient() {
    this.#disconnectClient();

    const options: GateKeeperBrowserClientOptions = {
      initialSession: this.#initialSession ?? parseSessionAttribute(this.getAttribute('initial-session')),
      sessionEndpoint: this.getAttribute('session-endpoint') ?? undefined,
      profileEndpoint: this.getAttribute('profile-endpoint') ?? undefined,
      refreshSkewSeconds: parseNumberAttribute(this.getAttribute('refresh-skew-seconds'), 120),
      minRefreshDelaySeconds: parseNumberAttribute(this.getAttribute('min-refresh-delay-seconds'), 5),
      autoRefresh: parseBooleanAttribute(this.getAttribute('auto-refresh'), true)
    };

    this.#client = createGateKeeperBrowserClient(options);
    this.#unsubscribe = this.#client.subscribe((snapshot) => this.#publishSnapshot(snapshot));
  }

  #disconnectClient() {
    this.#unsubscribe?.();
    this.#unsubscribe = null;
    this.#client?.stopAutoRefresh();
    this.#client = null;
  }

  #publishSnapshot(snapshot: GateKeeperBrowserSnapshot) {
    this.dataset.authenticated = String(snapshot.authenticated);
    this.dispatchEvent(
      new CustomEvent('gatekeeper:change', {
        bubbles: true,
        composed: true,
        detail: snapshot
      })
    );
  }
}

export function getClosestGateKeeperClient(element: Element): GateKeeperBrowserClient | null {
  const provider = element.closest('gatekeeper-provider');
  return provider instanceof GateKeeperProviderElement ? provider.client : null;
}
