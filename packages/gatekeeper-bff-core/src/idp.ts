import type { GateKeeperSession, LocalsLike, ResolvedGateKeeperConfig } from './types.js';

export class GateKeeperHttpError extends Error {
  status: number;
  detail: unknown;

  constructor(status: number, message: string, detail?: unknown) {
    super(message);
    this.name = 'GateKeeperHttpError';
    this.status = status;
    this.detail = detail;
  }
}

function resolveSession(
  sessionOrLocals: GateKeeperSession | LocalsLike | null | undefined
): GateKeeperSession | null {
  if (sessionOrLocals && 'accessToken' in sessionOrLocals) return sessionOrLocals;
  return sessionOrLocals?.session ?? null;
}

export async function idpFetch<T = unknown>(
  cfg: ResolvedGateKeeperConfig,
  sessionOrLocals: GateKeeperSession | LocalsLike | null | undefined,
  path: string,
  init: RequestInit = {}
): Promise<T> {
  const session = resolveSession(sessionOrLocals);

  if (!session?.accessToken) {
    throw new GateKeeperHttpError(401, 'Not signed in');
  }

  const headers = new Headers(init.headers);
  headers.set('Authorization', `Bearer ${session.accessToken}`);
  if (init.body && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }

  const res = await fetch(`${cfg.idpBaseUrl}${path}`, {
    ...init,
    headers
  });

  if (!res.ok) {
    const detail = await res.json().catch(() => null);
    const message =
      typeof detail === 'object' && detail !== null
        ? ((detail as { error?: string; errors?: Array<{ message?: string }> }).error ??
          (detail as { errors?: Array<{ message?: string }> }).errors?.[0]?.message ??
          `IdP request failed (${res.status})`)
        : `IdP request failed (${res.status})`;
    throw new GateKeeperHttpError(res.status, message, detail);
  }

  if (res.status === 204) return null as T;
  return (await res.json()) as T;
}
