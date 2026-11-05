import { idpFetch as fetchFromIdp } from './idp.js';
import { generatePkce } from './pkce.js';
import { createSessionStore } from './session.js';
import {
  buildAuthorizeUrl,
  decodeJwtPayload,
  exchangeCode,
  fetchUserInfo,
  profileFromClaims,
  refreshTokens,
  sessionFromTokenResponse,
  validateConfig
} from './oauth.js';
import { sessionWithUpdatedProfile, updateMyProfile } from './user.js';
import type { GateKeeperClient, GateKeeperConfig } from './types.js';

export function createGateKeeperClient(config: GateKeeperConfig): GateKeeperClient {
  const cfg = validateConfig(config);
  const sessions = createSessionStore(cfg);

  return {
    config: { ...cfg, clientSecret: undefined, sessionSecret: undefined },
    authRequestTtlSeconds: cfg.authRequestTtlSeconds,
    sessionCookieName: sessions.sessionCookieName,
    generatePkce,
    buildAuthorizeUrl: (input) => buildAuthorizeUrl(cfg, input),
    decodeJwtPayload,
    exchangeCode: (input) => exchangeCode(cfg, input),
    refreshTokens: (refreshToken) => refreshTokens(cfg, refreshToken),
    fetchUserInfo: (accessToken) => fetchUserInfo(cfg, accessToken),
    profileFromClaims,
    sessionFromTokenResponse,
    updateMyProfile: (sessionOrLocals, input) => updateMyProfile(cfg, sessionOrLocals, input),
    updateUserProfile: (sessionOrLocals, input) => updateMyProfile(cfg, sessionOrLocals, input),
    sessionWithUpdatedProfile,
    idpFetch: (sessionOrLocals, path, init) => fetchFromIdp(cfg, sessionOrLocals, path, init),
    encryptSession: sessions.encryptSession,
    decryptSession: sessions.decryptSession,
    setSession: sessions.setSession,
    readSession: sessions.readSession,
    clearSession: sessions.clearSession
  };
}
