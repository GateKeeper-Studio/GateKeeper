export { GATEKEEPER_CONFIG, GateKeeperService } from './gatekeeper.service';
export { provideGateKeeper } from './provide';
export { authGuard } from './auth.guard';
export type {
  GateKeeperProfile,
  GateKeeperSpaClient,
  GateKeeperSpaConfig,
  GateKeeperSpaSnapshot,
  GateKeeperSpaStatus,
  SignInOptions
} from '@gatekeeper/spa-oauth';
