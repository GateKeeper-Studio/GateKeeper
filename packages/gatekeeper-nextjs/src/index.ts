import { cookies } from 'next/headers';
import { NextResponse, type NextRequest } from 'next/server';
import {
  createGateKeeperBff,
  type BffOptions,
  type CookiesLike,
  type GateKeeperBff,
  type GateKeeperConfig,
  type GateKeeperProfile,
  type GateKeeperSession
} from '@gatekeeper/bff-core';

// Structural view of Next's cookie stores (works across Next 14/15).
interface NextMutableCookieStore {
  get(name: string): { value: string } | undefined;
  set(name: string, value: string, options?: Record<string, unknown>): void;
}

function toCookiesLike(store: NextMutableCookieStore): CookiesLike {
  return {
    get: (name) => store.get(name)?.value,
    set: (name, value, options) =>
      store.set(name, value, {
        httpOnly: options.httpOnly,
        sameSite: options.sameSite,
        secure: options.secure,
        path: options.path,
        maxAge: options.maxAge
      }),
    delete: (name, options) => store.set(name, '', { path: options.path, maxAge: 0 })
  };
}

export interface NextGateKeeper {
  bff: GateKeeperBff;
  /** Route handler: `export const GET = gk.signInHandler;` at app/auth/sign-in/route.ts */
  signInHandler(request: NextRequest): Promise<NextResponse>;
  /** Route handler: `export const GET = gk.callbackHandler;` at app/auth/callback/route.ts */
  callbackHandler(request: NextRequest): Promise<NextResponse>;
  /** Route handler: `export const POST = gk.signOutHandler;` at app/auth/sign-out/route.ts */
  signOutHandler(request: NextRequest): Promise<NextResponse>;
  /** middleware.ts: silent, coalesced refresh; writes the rotated cookie to the response. */
  middleware(request: NextRequest): Promise<NextResponse>;
  /** RSC-safe read (no refresh — refresh happens in middleware). */
  getSession(): Promise<GateKeeperSession | null>;
  getUser(): Promise<GateKeeperProfile | null>;
}

export function createNextGateKeeper(
  config: GateKeeperConfig,
  options: BffOptions = {}
): NextGateKeeper {
  const bff = createGateKeeperBff(config, options);

  async function store(): Promise<CookiesLike> {
    // Next 15 makes cookies() async; awaiting a non-promise (Next 14) is a no-op.
    return toCookiesLike((await cookies()) as unknown as NextMutableCookieStore);
  }

  async function signInHandler(request: NextRequest): Promise<NextResponse> {
    const returnTo = request.nextUrl.searchParams.get('returnTo') || '/';
    const { authorizeUrl } = bff.startSignIn(await store(), { returnTo });
    return NextResponse.redirect(authorizeUrl);
  }

  async function callbackHandler(request: NextRequest): Promise<NextResponse> {
    const code = request.nextUrl.searchParams.get('code');
    const state = request.nextUrl.searchParams.get('state');
    if (!code || !state) {
      return NextResponse.redirect(new URL('/?error=missing_code', request.url));
    }
    try {
      const { returnTo } = await bff.completeSignIn(await store(), { code, state });
      return NextResponse.redirect(new URL(returnTo, request.url));
    } catch {
      return NextResponse.redirect(new URL('/?error=auth', request.url));
    }
  }

  async function signOutHandler(request: NextRequest): Promise<NextResponse> {
    bff.signOut(await store());
    return NextResponse.redirect(new URL('/', request.url));
  }

  async function middleware(request: NextRequest): Promise<NextResponse> {
    const response = NextResponse.next();
    const cookieJar: CookiesLike = {
      get: (name) => request.cookies.get(name)?.value,
      set: (name, value, options) =>
        response.cookies.set(name, value, {
          httpOnly: options.httpOnly,
          sameSite: options.sameSite,
          secure: options.secure,
          path: options.path,
          maxAge: options.maxAge
        }),
      delete: (name, options) => response.cookies.set(name, '', { path: options.path, maxAge: 0 })
    };
    await bff.loadSession(cookieJar);
    return response;
  }

  async function getSession(): Promise<GateKeeperSession | null> {
    // Read-only: safe inside Server Components (never mutates cookies here).
    return bff.readSession(await store());
  }

  async function getUser(): Promise<GateKeeperProfile | null> {
    return (await getSession())?.profile ?? null;
  }

  return {
    bff,
    signInHandler,
    callbackHandler,
    signOutHandler,
    middleware,
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
