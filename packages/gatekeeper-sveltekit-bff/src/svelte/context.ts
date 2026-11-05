import { getContext, hasContext } from 'svelte';
import type { BrowserSafeSession, GateKeeperProfile } from '../types.js';

export const GATEKEEPER_CONTEXT_KEY = Symbol('gatekeeper');

export interface GateKeeperContext {
  readonly session: BrowserSafeSession | null;
  readonly user: GateKeeperProfile | null;
  readonly authenticated: boolean;
  readonly expiresAt: number | null;
  readonly refreshing: boolean;
  readonly error: unknown;
  refresh(): Promise<BrowserSafeSession | null>;
}

export function getGateKeeperContext(): GateKeeperContext {
  return getContext<GateKeeperContext>(GATEKEEPER_CONTEXT_KEY);
}

export function hasGateKeeperContext(): boolean {
  return hasContext(GATEKEEPER_CONTEXT_KEY);
}
