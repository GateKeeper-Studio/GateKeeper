import { computed, inject, type ComputedRef } from 'vue';
import type {
  GateKeeperProfile,
  GateKeeperSpaClient,
  GateKeeperSpaStatus,
  SignInOptions
} from '@gatekeeper/spa-oauth';
import { GATEKEEPER_INJECTION_KEY } from './plugin.js';

export interface UseGateKeeper {
  status: ComputedRef<GateKeeperSpaStatus>;
  isAuthenticated: ComputedRef<boolean>;
  isLoading: ComputedRef<boolean>;
  user: ComputedRef<GateKeeperProfile | null>;
  expiresAt: ComputedRef<number | null>;
  error: ComputedRef<Error | null>;
  signIn: (options?: SignInOptions) => Promise<void>;
  signOut: () => void;
  refresh: () => Promise<void>;
  getAccessToken: () => Promise<string | null>;
  client: GateKeeperSpaClient;
}

export function useGateKeeper(): UseGateKeeper {
  const instance = inject(GATEKEEPER_INJECTION_KEY);
  if (!instance) {
    throw new Error('useGateKeeper() requires the GateKeeper plugin: app.use(createGateKeeper(...)).');
  }

  const { client, state } = instance;

  return {
    status: computed(() => state.value.status),
    isAuthenticated: computed(() => state.value.authenticated),
    isLoading: computed(() => state.value.status === 'loading'),
    user: computed(() => state.value.user),
    expiresAt: computed(() => state.value.expiresAt),
    error: computed(() => state.value.error),
    signIn: (options?: SignInOptions) => client.signIn(options),
    signOut: () => client.signOut(),
    refresh: () => client.refresh(),
    getAccessToken: () => client.getAccessToken(),
    client
  };
}
