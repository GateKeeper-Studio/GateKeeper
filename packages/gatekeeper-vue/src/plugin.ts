import { shallowRef, type App, type InjectionKey, type Ref } from 'vue';
import {
  createGateKeeperSpaClient,
  type GateKeeperSpaClient,
  type GateKeeperSpaConfig,
  type GateKeeperSpaSnapshot
} from '@gatekeeper/spa-oauth';

export interface GateKeeperVueInstance {
  client: GateKeeperSpaClient;
  state: Ref<GateKeeperSpaSnapshot>;
}

export const GATEKEEPER_INJECTION_KEY: InjectionKey<GateKeeperVueInstance> = Symbol('gatekeeper');

export interface CreateGateKeeperOptions {
  /** Config to build a client, OR pass a pre-built `client`. */
  config?: GateKeeperSpaConfig;
  /** A pre-built client (takes precedence over `config`). */
  client?: GateKeeperSpaClient;
  /** Finish the flow if the URL carries `?code=` on install. Defaults to true. */
  handleRedirectCallback?: boolean;
}

export interface GateKeeperPlugin {
  install(app: App): void;
  client: GateKeeperSpaClient;
}

/** `app.use(createGateKeeper({ config }))` installs GateKeeper into a Vue app. */
export function createGateKeeper(options: CreateGateKeeperOptions): GateKeeperPlugin {
  const client =
    options.client ??
    (() => {
      if (!options.config) throw new Error('createGateKeeper requires either `client` or `config`.');
      return createGateKeeperSpaClient(options.config);
    })();

  const state = shallowRef<GateKeeperSpaSnapshot>(client.getSnapshot());
  client.subscribe((snapshot) => {
    state.value = snapshot;
  });

  return {
    client,
    install(app: App) {
      if (options.handleRedirectCallback !== false) {
        client.handleRedirectCallback().catch(() => {
          /* surfaced via state.error */
        });
      }
      app.provide(GATEKEEPER_INJECTION_KEY, { client, state });
    }
  };
}
