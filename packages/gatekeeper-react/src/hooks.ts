import { useCallback } from 'react';
import type {
  GateKeeperProfile,
  GateKeeperSpaClient,
  GateKeeperSpaStatus,
  SignInOptions
} from '@gatekeeper/spa-oauth';
import { useGateKeeperContext } from './context.js';

export interface UseGateKeeper {
  status: GateKeeperSpaStatus;
  isAuthenticated: boolean;
  isLoading: boolean;
  user: GateKeeperProfile | null;
  expiresAt: number | null;
  error: Error | null;
  signIn: (options?: SignInOptions) => Promise<void>;
  signOut: () => void;
  refresh: () => Promise<void>;
  getAccessToken: () => Promise<string | null>;
  client: GateKeeperSpaClient;
}

export function useGateKeeper(): UseGateKeeper {
  const { client, snapshot } = useGateKeeperContext();

  const signIn = useCallback((options?: SignInOptions) => client.signIn(options), [client]);
  const signOut = useCallback(() => client.signOut(), [client]);
  const refresh = useCallback(() => client.refresh(), [client]);
  const getAccessToken = useCallback(() => client.getAccessToken(), [client]);

  return {
    status: snapshot.status,
    isAuthenticated: snapshot.authenticated,
    isLoading: snapshot.status === 'loading',
    user: snapshot.user,
    expiresAt: snapshot.expiresAt,
    error: snapshot.error,
    signIn,
    signOut,
    refresh,
    getAccessToken,
    client
  };
}

export function useUser(): GateKeeperProfile | null {
  return useGateKeeperContext().snapshot.user;
}
