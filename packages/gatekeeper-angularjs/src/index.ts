import * as angular from 'angular';
import {
  createGateKeeperSpaClient,
  type GateKeeperProfile,
  type GateKeeperSpaClient,
  type GateKeeperSpaConfig,
  type GateKeeperSpaSnapshot,
  type GateKeeperSpaStatus,
  type JwtClaims,
  type SignInOptions
} from '@gatekeeper/spa-oauth';

/** AngularJS module name to depend on: `angular.module('app', [GATEKEEPER_MODULE])`. */
export const GATEKEEPER_MODULE = 'gatekeeper';

/** The `gatekeeper` service injected into controllers/components. */
export interface GateKeeperNg1Service {
  readonly snapshot: GateKeeperSpaSnapshot;
  isAuthenticated(): boolean;
  user(): GateKeeperProfile | null;
  status(): GateKeeperSpaStatus;
  error(): Error | null;
  expiresAt(): number | null;
  signIn(options?: SignInOptions): Promise<void>;
  signOut(): void;
  refresh(): Promise<void>;
  getAccessToken(): Promise<string | null>;
  fetchUserInfo(): Promise<JwtClaims>;
  client: GateKeeperSpaClient;
}

/** The provider you configure in a `.config()` block. */
export interface GateKeeperNg1Provider extends angular.IServiceProvider {
  configure(config: GateKeeperSpaConfig): void;
}

function buildService(
  config: GateKeeperSpaConfig,
  $rootScope: angular.IRootScopeService
): GateKeeperNg1Service {
  const client = createGateKeeperSpaClient(config);
  let snapshot = client.getSnapshot();
  client.subscribe((next) => {
    snapshot = next;
    // Bring the new state into an AngularJS digest.
    $rootScope.$applyAsync();
  });
  // Finish the flow if we were redirected back with ?code=.
  client.handleRedirectCallback().catch(() => {
    /* surfaced via error() */
  });

  return {
    get snapshot() {
      return snapshot;
    },
    isAuthenticated: () => snapshot.authenticated,
    user: () => snapshot.user,
    status: () => snapshot.status,
    error: () => snapshot.error,
    expiresAt: () => snapshot.expiresAt,
    signIn: (options?: SignInOptions) => client.signIn(options),
    signOut: () => client.signOut(),
    refresh: () => client.refresh(),
    getAccessToken: () => client.getAccessToken(),
    fetchUserInfo: () => client.fetchUserInfo(),
    client
  };
}

// AngularJS instantiates providers with `new`, so this must be a constructor.
class GateKeeperProviderImpl implements GateKeeperNg1Provider {
  private config: GateKeeperSpaConfig | null = null;

  configure(value: GateKeeperSpaConfig): void {
    this.config = value;
  }

  $get = [
    '$rootScope',
    ($rootScope: angular.IRootScopeService): GateKeeperNg1Service => {
      if (!this.config) {
        throw new Error('gatekeeperProvider.configure({...}) must be called in a .config() block.');
      }
      return buildService(this.config, $rootScope);
    }
  ];
}

/**
 * ngRoute/UI-Router resolve that blocks a route until the user is authenticated,
 * kicking off sign-in otherwise:
 *
 * ```js
 * $routeProvider.when('/account', { resolve: { auth: requireAuthenticated } });
 * ```
 */
export const requireAuthenticated = [
  'gatekeeper',
  function requireAuthenticatedResolve(gatekeeper: GateKeeperNg1Service): boolean {
    if (gatekeeper.isAuthenticated()) return true;
    void gatekeeper.signIn();
    return false;
  }
];

/** The registered AngularJS module. Import for its side effect or add by name. */
export const gatekeeperModule: angular.IModule = angular
  .module(GATEKEEPER_MODULE, [])
  .provider('gatekeeper', GateKeeperProviderImpl);

export type {
  GateKeeperProfile,
  GateKeeperSpaClient,
  GateKeeperSpaConfig,
  GateKeeperSpaSnapshot,
  GateKeeperSpaStatus,
  SignInOptions
} from '@gatekeeper/spa-oauth';
