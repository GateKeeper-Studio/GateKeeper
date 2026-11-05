import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useSyncExternalStore,
  type ReactNode
} from 'react';
import {
  createGateKeeperSpaClient,
  type GateKeeperSpaClient,
  type GateKeeperSpaConfig,
  type GateKeeperSpaSnapshot
} from '@gatekeeper/spa-oauth';

interface GateKeeperContextValue {
  client: GateKeeperSpaClient;
  snapshot: GateKeeperSpaSnapshot;
}

const GateKeeperContext = createContext<GateKeeperContextValue | null>(null);

export interface GateKeeperProviderProps {
  /** Config to build a client, OR pass a pre-built `client`. */
  config?: GateKeeperSpaConfig;
  /** A pre-built client (takes precedence over `config`). */
  client?: GateKeeperSpaClient;
  /**
   * Automatically finish the flow if the URL carries `?code=` on mount.
   * Defaults to true.
   */
  handleRedirectCallback?: boolean;
  children: ReactNode;
}

export function GateKeeperProvider({
  config,
  client: providedClient,
  handleRedirectCallback = true,
  children
}: GateKeeperProviderProps): JSX.Element {
  const client = useMemo(() => {
    if (providedClient) return providedClient;
    if (!config) throw new Error('GateKeeperProvider requires either `client` or `config`.');
    return createGateKeeperSpaClient(config);
  }, [providedClient, config]);

  const handledRef = useRef(false);
  useEffect(() => {
    if (!handleRedirectCallback || handledRef.current) return;
    handledRef.current = true;
    client.handleRedirectCallback().catch(() => {
      /* surfaced via snapshot.error */
    });
  }, [client, handleRedirectCallback]);

  const snapshot = useSyncExternalStore(
    client.subscribe,
    client.getSnapshot,
    client.getSnapshot
  );

  const value = useMemo<GateKeeperContextValue>(() => ({ client, snapshot }), [client, snapshot]);

  return <GateKeeperContext.Provider value={value}>{children}</GateKeeperContext.Provider>;
}

export function useGateKeeperContext(): GateKeeperContextValue {
  const value = useContext(GateKeeperContext);
  if (!value) {
    throw new Error('useGateKeeper must be used within a <GateKeeperProvider>.');
  }
  return value;
}
