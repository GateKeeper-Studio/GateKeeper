// Home page (server side). When signed in, fetch the account's enrolled MFA
// factors so the sign-in hint checkboxes only offer methods that actually exist
// — the IdP can't challenge a factor that was never set up.
//
// Hints are UX-only (the IdP still enforces the real policy), so a failed lookup
// degrades gracefully to "no factors known" rather than breaking the home page.
import { idpFetch } from '$lib/server/gatekeeper.js';
import type { PageServerLoad } from './$types';

interface MfaMethodsResponse {
  methods?: Array<{ method?: string; confirmed?: boolean }>;
}

export const load: PageServerLoad = async ({ locals }) => {
  if (!locals.session) return { enrolledMfaMethods: [] };

  try {
    const mfa = await idpFetch<MfaMethodsResponse>(locals, '/me/mfa/methods');
    // Only confirmed factors are usable at sign-in: this includes the
    // always-available email code and excludes a pending (unconfirmed) TOTP.
    const enrolled = [
      ...new Set((mfa?.methods ?? []).filter((m) => m.confirmed).map((m) => m.method))
    ];
    return { enrolledMfaMethods: enrolled };
  } catch {
    return { enrolledMfaMethods: [] };
  }
};
