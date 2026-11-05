import { requestJson } from './http.js';
import type {
  BrowserSafeSession,
  GateKeeperBrowserClient,
  GateKeeperBrowserClientOptions,
  GateKeeperBrowserEventHandler,
  GateKeeperBrowserEventName,
  GateKeeperBrowserSnapshot,
  GateKeeperBrowserState,
  GateKeeperBrowserSubscriber,
  GateKeeperProfile,
  UpdateUserProfileInput,
  UserProfileView
} from './types.js';

function snapshotFromState(state: GateKeeperBrowserState): GateKeeperBrowserSnapshot {
  return {
    ...state,
    user: state.session?.user ?? null,
    authenticated: Boolean(state.session?.authenticated),
    expiresAt: state.session?.expiresAt ?? null
  };
}

export function createGateKeeperBrowserClient(
  options: GateKeeperBrowserClientOptions = {}
): GateKeeperBrowserClient {
  const sessionEndpoint = options.sessionEndpoint ?? '/api/gatekeeper/session';
  const profileEndpoint = options.profileEndpoint ?? '/api/me/profile';
  const fetchFn = options.fetch ?? fetch;
  const refreshSkewSeconds = options.refreshSkewSeconds ?? 120;
  const minRefreshDelaySeconds = options.minRefreshDelaySeconds ?? 5;
  const subscribers = new Set<GateKeeperBrowserSubscriber>();
  const eventHandlers: Record<GateKeeperBrowserEventName, Set<GateKeeperBrowserEventHandler>> = {
    change: new Set(),
    refresh: new Set(),
    error: new Set()
  };

  let state: GateKeeperBrowserState = {
    session: options.initialSession ?? null,
    refreshing: false,
    error: null
  };
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;
  let autoRefreshEnabled = false;
  let inFlightRefresh: Promise<BrowserSafeSession | null> | null = null;

  function getSnapshot(): GateKeeperBrowserSnapshot {
    return snapshotFromState(state);
  }

  function emit(event: GateKeeperBrowserEventName) {
    const snapshot = getSnapshot();
    for (const subscriber of subscribers) subscriber(snapshot);
    for (const handler of eventHandlers[event]) handler(snapshot);
    if (event !== 'change') {
      for (const handler of eventHandlers.change) handler(snapshot);
    }
  }

  function setState(next: Partial<GateKeeperBrowserState>, event: GateKeeperBrowserEventName = 'change') {
    state = { ...state, ...next };
    emit(event);
    scheduleRefresh();
  }

  function clearRefreshTimer() {
    if (refreshTimer) {
      clearTimeout(refreshTimer);
      refreshTimer = undefined;
    }
  }

  function scheduleRefresh() {
    clearRefreshTimer();
    if (!autoRefreshEnabled || !state.session?.authenticated || !state.session.expiresAt) return;

    const now = Math.floor(Date.now() / 1000);
    const refreshAt = state.session.expiresAt - refreshSkewSeconds;
    const delayMs = Math.max((refreshAt - now) * 1000, minRefreshDelaySeconds * 1000);

    refreshTimer = setTimeout(() => {
      client.refresh().catch(() => {
        // Consumers can subscribe to the error event; stale browser-safe state remains readable.
      });
    }, delayMs);
  }

  const client: GateKeeperBrowserClient = {
    getSnapshot,
    getSession() {
      return state.session;
    },
    getUser(): GateKeeperProfile | null {
      return state.session?.user ?? null;
    },
    getAccessToken() {
      return state.session?.accessToken ?? null;
    },
    isAuthenticated() {
      return Boolean(state.session?.authenticated);
    },
    setSession(session) {
      setState({ session, error: null });
    },
    async refresh() {
      if (inFlightRefresh) return inFlightRefresh;

      setState({ refreshing: true }, 'refresh');
      inFlightRefresh = requestJson<BrowserSafeSession>(sessionEndpoint, {
        method: 'GET',
        fetch: fetchFn
      })
        .then((session) => {
          setState({ session, error: null, refreshing: false }, 'refresh');
          return session;
        })
        .catch((error: unknown) => {
          setState({ error, refreshing: false }, 'error');
          throw error;
        })
        .finally(() => {
          inFlightRefresh = null;
        });

      return inFlightRefresh;
    },
    startAutoRefresh() {
      autoRefreshEnabled = true;
      scheduleRefresh();
    },
    stopAutoRefresh() {
      autoRefreshEnabled = false;
      clearRefreshTimer();
    },
    async updateUserProfile(input: UpdateUserProfileInput) {
      const profile = await requestJson<UserProfileView>(profileEndpoint, {
        method: 'PUT',
        body: JSON.stringify(input),
        fetch: fetchFn
      });
      await this.refresh().catch(() => null);
      return profile;
    },
    subscribe(subscriber) {
      subscribers.add(subscriber);
      subscriber(getSnapshot());
      return () => subscribers.delete(subscriber);
    },
    on(event, handler) {
      eventHandlers[event].add(handler);
      return () => eventHandlers[event].delete(handler);
    }
  };

  if (options.autoRefresh ?? true) {
    client.startAutoRefresh();
  }

  return client;
}
