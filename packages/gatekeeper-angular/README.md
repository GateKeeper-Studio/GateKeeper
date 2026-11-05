# @gatekeeper/angular

Angular (2+, standalone-API) bindings for the GateKeeper **public client**
(Authorization Code + PKCE, no client secret). Exposes a signal-based
`GateKeeperService`, a `provideGateKeeper()` provider, and an `authGuard`.

Shipped as **TypeScript source** — consume it from a monorepo `paths` alias (or
package it with `ng-packagr` for publishing). It builds on `@gatekeeper/spa-oauth`.

## Setup

```ts
// main.ts
import { bootstrapApplication } from '@angular/platform-browser';
import { provideRouter } from '@angular/router';
import { provideGateKeeper } from '@gatekeeper/angular';
import { AppComponent } from './app/app.component';

bootstrapApplication(AppComponent, {
  providers: [
    provideRouter([/* ... */]),
    provideGateKeeper({
      idpBaseUrl: 'http://localhost:8080',
      authUiBaseUrl: 'http://localhost:5173',
      tenantId: '<tenant-uuid>',
      clientId: '<public-client-id>',
      redirectUri: 'http://localhost:5177/'
    })
  ]
});
```

`provideGateKeeper` also runs `handleRedirectCallback()` at startup (via
`APP_INITIALIZER`) to finish the flow after the redirect back.

## Use in a component

```ts
import { Component, inject } from '@angular/core';
import { GateKeeperService } from '@gatekeeper/angular';

@Component({
  standalone: true,
  selector: 'app-nav',
  template: `
    @if (gk.isAuthenticated()) {
      <span>{{ gk.user()?.email }}</span>
      <button (click)="gk.signOut()">Sign out</button>
    } @else {
      <button (click)="gk.signIn()">Sign in</button>
    }
  `
})
export class NavComponent {
  gk = inject(GateKeeperService);
}
```

Signals: `status()`, `isAuthenticated()`, `user()`, `expiresAt()`, `error()`.
Methods: `signIn()`, `signOut()`, `refresh()`, `getAccessToken()`, `fetchUserInfo()`.

## Route guard

```ts
import { authGuard } from '@gatekeeper/angular';
export const routes = [{ path: 'account', canActivate: [authGuard], loadComponent: /* ... */ }];
```

## Public client only

Tokens live in the browser (`sessionStorage`). Register with `client_type: "public"`
(no secret) and add the app origin to the IdP's `CORS_ALLOWED_ORIGINS`.
