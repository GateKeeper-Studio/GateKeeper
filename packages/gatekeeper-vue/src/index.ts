export { createGateKeeper, GATEKEEPER_INJECTION_KEY } from './plugin.js';
export type {
  CreateGateKeeperOptions,
  GateKeeperPlugin,
  GateKeeperVueInstance
} from './plugin.js';
export { useGateKeeper } from './composable.js';
export type { UseGateKeeper } from './composable.js';
export type {
  GateKeeperProfile,
  GateKeeperSpaClient,
  GateKeeperSpaConfig,
  GateKeeperSpaSnapshot,
  GateKeeperSpaStatus,
  SignInOptions
} from '@gatekeeper/spa-oauth';
