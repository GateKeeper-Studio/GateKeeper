"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

type SessionContextType = {
  accessToken: string;
  sessionExpired: boolean;
};

const SessionContext = createContext<SessionContextType>(
  {} as SessionContextType,
);

type SessionProviderProps = {
  children: ReactNode;
  initialAccessToken: string;
};

function getTokenExp(token: string): number | null {
  try {
    const [, payload] = token.split(".");
    const decoded = JSON.parse(atob(payload));
    return typeof decoded.exp === "number" ? decoded.exp : null;
  } catch {
    return null;
  }
}

/**
 * Provides the current access token to the profile page tree.
 *
 * Proactively refreshes the token 2 minutes before expiry by calling
 * the `/api/auth/refresh` route. If the refresh fails (e.g. the token
 * is already too old), sets `sessionExpired` so the UI can react.
 */
export function SessionProvider({
  children,
  initialAccessToken,
}: SessionProviderProps) {
  const [accessToken, setAccessToken] = useState(initialAccessToken);
  const [sessionExpired, setSessionExpired] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const refresh = useCallback(async () => {
    try {
      const res = await fetch("/api/auth/refresh", { method: "POST" });

      if (!res.ok) {
        setSessionExpired(true);
        return;
      }

      const data = (await res.json()) as {
        accessToken: string;
        expiresIn: number;
      };

      setAccessToken(data.accessToken);
      setSessionExpired(false);
    } catch {
      setSessionExpired(true);
    }
  }, []);

  // Schedule a refresh 2 minutes before the token expires.
  // Re-runs every time `accessToken` changes (including after a successful refresh).
  useEffect(() => {
    if (sessionExpired) return;

    const exp = getTokenExp(accessToken);
    if (!exp) return;

    const nowSec = Math.floor(Date.now() / 1000);
    const remainingSec = exp - nowSec;

    // If the token is already expired, try to refresh immediately
    if (remainingSec <= 0) {
      refresh();
      return;
    }

    // Schedule refresh 2 minutes before expiry (minimum 10 seconds)
    const refreshInSec = Math.max(remainingSec - 120, 10);

    timerRef.current = setTimeout(() => {
      refresh();
    }, refreshInSec * 1000);

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [accessToken, sessionExpired, refresh]);

  // Also handle tab becoming visible again after being inactive
  useEffect(() => {
    function handleVisibilityChange() {
      if (document.visibilityState !== "visible" || sessionExpired) return;

      const exp = getTokenExp(accessToken);
      if (!exp) return;

      const nowSec = Math.floor(Date.now() / 1000);
      const remainingSec = exp - nowSec;

      // If token expired while tab was hidden, refresh immediately
      if (remainingSec <= 120) {
        refresh();
      }
    }

    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () =>
      document.removeEventListener("visibilitychange", handleVisibilityChange);
  }, [accessToken, sessionExpired, refresh]);

  return (
    <SessionContext.Provider value={{ accessToken, sessionExpired }}>
      {children}
    </SessionContext.Provider>
  );
}

export const useSession = () => useContext(SessionContext);
