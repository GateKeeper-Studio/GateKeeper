import { Inject, Injectable, InjectionToken, computed, signal, type Signal } from '@angular/core';
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

/** Provide a `GateKeeperSpaConfig` under this token (see `provideGateKeeper`). */
export const GATEKEEPER_CONFIG = new InjectionToken<GateKeeperSpaConfig>('GATEKEEPER_CONFIG');

/**
 * Root-provided service wrapping the framework-agnostic public SPA client. State
 * is exposed as Angular signals so templates react without manual subscriptions.
 */
@Injectable({ providedIn: 'root' })
export class GateKeeperService {
  private readonly client: GateKeeperSpaClient;
  private readonly snapshotSignal;

  readonly snapshot: Signal<GateKeeperSpaSnapshot>;
  readonly status: Signal<GateKeeperSpaStatus>;
  readonly isAuthenticated: Signal<boolean>;
  readonly user: Signal<GateKeeperProfile | null>;
  readonly expiresAt: Signal<number | null>;
  readonly error: Signal<Error | null>;

  constructor(@Inject(GATEKEEPER_CONFIG) config: GateKeeperSpaConfig) {
    this.client = createGateKeeperSpaClient(config);
    this.snapshotSignal = signal<GateKeeperSpaSnapshot>(this.client.getSnapshot());
    this.snapshot = this.snapshotSignal.asReadonly();
    this.status = computed(() => this.snapshotSignal().status);
    this.isAuthenticated = computed(() => this.snapshotSignal().authenticated);
    this.user = computed(() => this.snapshotSignal().user);
    this.expiresAt = computed(() => this.snapshotSignal().expiresAt);
    this.error = computed(() => this.snapshotSignal().error);
    this.client.subscribe((next) => this.snapshotSignal.set(next));
  }

  signIn(options?: SignInOptions): Promise<void> {
    return this.client.signIn(options);
  }

  signOut(): void {
    this.client.signOut();
  }

  refresh(): Promise<void> {
    return this.client.refresh();
  }

  getAccessToken(): Promise<string | null> {
    return this.client.getAccessToken();
  }

  fetchUserInfo(): Promise<JwtClaims> {
    return this.client.fetchUserInfo();
  }

  /** Finish the flow when the URL carries `?code=`. Wired by `provideGateKeeper`. */
  handleRedirectCallback(): Promise<GateKeeperProfile | null> {
    return this.client.handleRedirectCallback();
  }
}
