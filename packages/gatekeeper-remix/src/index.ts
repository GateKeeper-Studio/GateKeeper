import {
  createGateKeeperBff,
  type BffOptions,
  type CookieOptions,
  type CookiesLike,
  type GateKeeperBff,
  type GateKeeperConfig,
  type GateKeeperProfile,
  type GateKeeperSession
} from '@gatekeeper/bff-core';

// Remix has no mutable cookie jar: cookies are read from the request's Cookie
// header and written via Set-Cookie response headers. This jar bridges both to the
// CookiesLike contract bff-core expects.

function parseCookieHeader(header: string | null): Record<string, string> {
  const out: Record<string, string> = {};
  if (!header) return out;
  for (const part of header.split(';')) {
    const eq = part.indexOf('=');
    if (eq === -1) continue;
    const name = part.slice(0, eq).trim();
    if (!name) continue;
    out[name] = decodeURIComponent(part.slice(eq + 1).trim());
  }
  return out;
}

function serializeCookie(name: string, value: string, options: CookieOptions): string {
  let str = `${name}=${encodeURIComponent(value)}`;
  if (options.maxAge !== undefined) str += `; Max-Age=${Math.floor(options.maxAge)}`;
  if (options.path) str += `; Path=${options.path}`;
  if (options.httpOnly) str += '; HttpOnly';
  if (options.secure) str += '; Secure';
  if (options.sameSite) {
    str += `; SameSite=${options.sameSite.charAt(0).toUpperCase()}${options.sameSite.slice(1)}`;
  }
  return str;
}

interface HeaderCookieJar {
  jar: CookiesLike;
  /** Accumulated Set-Cookie headers to merge into the response. */
  headers: Headers;
}

function createHeaderJar(request: Request): HeaderCookieJar {
  const incoming = parseCookieHeader(request.headers.get('cookie'));
  const headers = new Headers();
  const jar: CookiesLike = {
    get: (name) => incoming[name],
    set: (name, value, options) => headers.append('Set-Cookie', serializeCookie(name, value, options)),
    delete: (name, options) =>
      headers.append('Set-Cookie', serializeCookie(name, '', { path: options.path, maxAge: 0 }))
  };
  return { jar, headers };
}

function redirectResponse(location: string, headers: Headers): Response {
  headers.set('Location', location);
  return new Response(null, { status: 302, headers });
}

export interface RemixGateKeeper {
  bff: GateKeeperBff;
  /** loader for routes/auth.sign-in.tsx — redirects to the hosted login UI. */
  signInLoader(request: Request): Response;
  /** loader for routes/auth.callback.tsx — exchanges the code and seals the session. */
  callbackLoader(request: Request): Promise<Response>;
  /** action for routes/auth.sign-out.tsx — clears the session. */
  signOutAction(request: Request): Response;
  /**
   * Read + silently refresh the session (coalesced). Returns Set-Cookie `headers`
   * to merge into the loader's Response — call this from your root loader.
   */
  loadSession(request: Request): Promise<{ session: GateKeeperSession | null; headers: Headers }>;
  /** Read-only session (no refresh) — safe anywhere you don't return headers. */
  getSession(request: Request): GateKeeperSession | null;
  getUser(request: Request): GateKeeperProfile | null;
}

export function createRemixGateKeeper(
  config: GateKeeperConfig,
  options: BffOptions = {}
): RemixGateKeeper {
  const bff = createGateKeeperBff(config, options);

  function signInLoader(request: Request): Response {
    const url = new URL(request.url);
    const returnTo = url.searchParams.get('returnTo') || '/';
    const { jar, headers } = createHeaderJar(request);
    const { authorizeUrl } = bff.startSignIn(jar, { returnTo });
    return redirectResponse(authorizeUrl, headers);
  }

  async function callbackLoader(request: Request): Promise<Response> {
    const url = new URL(request.url);
    const code = url.searchParams.get('code');
    const state = url.searchParams.get('state');
    const { jar, headers } = createHeaderJar(request);
    if (!code || !state) return redirectResponse('/?error=missing_code', headers);
    try {
      const { returnTo } = await bff.completeSignIn(jar, { code, state });
      return redirectResponse(returnTo, headers);
    } catch {
      return redirectResponse('/?error=auth', headers);
    }
  }

  function signOutAction(request: Request): Response {
    const { jar, headers } = createHeaderJar(request);
    bff.signOut(jar);
    return redirectResponse('/', headers);
  }

  async function loadSession(
    request: Request
  ): Promise<{ session: GateKeeperSession | null; headers: Headers }> {
    const { jar, headers } = createHeaderJar(request);
    const session = await bff.loadSession(jar);
    return { session, headers };
  }

  function getSession(request: Request): GateKeeperSession | null {
    const { jar } = createHeaderJar(request);
    return bff.readSession(jar);
  }

  function getUser(request: Request): GateKeeperProfile | null {
    return getSession(request)?.profile ?? null;
  }

  return {
    bff,
    signInLoader,
    callbackLoader,
    signOutAction,
    loadSession,
    getSession,
    getUser
  };
}

export type {
  BffOptions,
  BrowserSafeSession,
  GateKeeperConfig,
  GateKeeperProfile,
  GateKeeperSession
} from '@gatekeeper/bff-core';
