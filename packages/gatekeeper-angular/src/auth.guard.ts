import { inject } from '@angular/core';
import type { CanActivateFn } from '@angular/router';
import { GateKeeperService } from './gatekeeper.service';

/**
 * Route guard: allow when authenticated, otherwise kick off sign-in and block
 * navigation. Attach to protected routes: `{ path: 'account', canActivate: [authGuard] }`.
 */
export const authGuard: CanActivateFn = () => {
  const gatekeeper = inject(GateKeeperService);
  if (gatekeeper.isAuthenticated()) return true;
  void gatekeeper.signIn();
  return false;
};
