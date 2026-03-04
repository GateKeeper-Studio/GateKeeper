import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";

interface SessionContextProps {
  session: GateKeeperSession | null;
  loading: boolean;
  error: GateKeeperSessionError | null;
  isAuthenticated: boolean;
  sessionExpired: boolean;
}

interface SessionProviderProps {
  children: React.ReactNode;
}

/** Decode a JWT payload without signature verification (for reading exp). */
function decodeJwtPayload(token: string): Record<string, unknown> | null {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return null;
    return JSON.parse(atob(parts[1].replace(/-/g, "+").replace(/_/g, "/")));
  } catch {
    return null;
  }
}

/** Returns seconds remaining until the token expires. */
function secondsUntilExpiry(token: string): number {
  const payload = decodeJwtPayload(token);
  if (!payload || typeof payload.exp !== "number") return 0;
  return payload.exp - Math.floor(Date.now() / 1000);
}

const sessionContext = createContext<SessionContextProps>(
  {} as SessionContextProps,
);

export function SessionProvider({ children }: SessionProviderProps) {
  const [session, setSession] = useState<GateKeeperSession | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<GateKeeperSessionError | null>(null);
  const [sessionExpired, setSessionExpired] = useState(false);

  const refreshTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearRefreshTimer = useCallback(() => {
    if (refreshTimerRef.current) {
      clearTimeout(refreshTimerRef.current);
      refreshTimerRef.current = null;
    }
  }, []);

  const refreshToken = useCallback(async () => {
    try {
      const res = await fetch("/api/session/refresh", { method: "POST" });

      if (!res.ok) {
        setSessionExpired(true);
        return;
      }

      const { session: newSession, expiresIn } = (await res.json()) as {
        session: GateKeeperSession;
        expiresIn: number;
      };

      setSession(newSession);
      setSessionExpired(false);

      // Schedule next refresh 2 minutes before expiry
      scheduleRefresh(expiresIn);
    } catch {
      setSessionExpired(true);
    }
  }, []);

  const scheduleRefresh = useCallback(
    (expiresInSec: number) => {
      clearRefreshTimer();

      // Refresh 2 minutes before expiry (minimum 10s)
      const delaySec = Math.max(expiresInSec - 120, 10);
      refreshTimerRef.current = setTimeout(refreshToken, delaySec * 1000);
    },
    [clearRefreshTimer, refreshToken],
  );

  // Initial session fetch
  const fetchSession = useCallback(() => {
    setLoading(true);

    fetch("/api/session")
      .then(async (res) => {
        if (!res.ok) {
          setError({ message: "Failed to load session" });
          return;
        }

        const data = (await res.json()) as GateKeeperSession;
        setSession(data);

        // Schedule first auto-refresh based on access token expiry
        if (data.accessToken) {
          const remaining = secondsUntilExpiry(data.accessToken);
          if (remaining <= 0) {
            // Token already expired — try to refresh immediately
            refreshToken();
          } else {
            scheduleRefresh(remaining);
          }
        }
      })
      .catch((err) => {
        setError({ message: err.message });
      })
      .finally(() => {
        setLoading(false);
      });
  }, [refreshToken, scheduleRefresh]);

  useEffect(() => {
    fetchSession();
    return clearRefreshTimer;
  }, [fetchSession, clearRefreshTimer]);

  // Refresh when tab comes back into focus
  useEffect(() => {
    const handleVisibilityChange = () => {
      if (
        document.visibilityState === "visible" &&
        session?.accessToken &&
        !sessionExpired
      ) {
        const remaining = secondsUntilExpiry(session.accessToken);
        if (remaining <= 120) {
          refreshToken();
        }
      }
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () =>
      document.removeEventListener("visibilitychange", handleVisibilityChange);
  }, [session, sessionExpired, refreshToken]);

  return (
    <sessionContext.Provider
      value={{
        error,
        isAuthenticated: !!session && !sessionExpired,
        loading,
        session,
        sessionExpired,
      }}
    >
      {children}
    </sessionContext.Provider>
  );
}

export const useSession = () => useContext(sessionContext);
