# @gatekeeper/angularjs

AngularJS (1.x) bindings for the GateKeeper **public client** (Authorization Code +
PKCE, no client secret). Registers a `gatekeeper` module with a configurable
provider, a `gatekeeper` service, and a `requireAuthenticated` route resolve.

Built on `@gatekeeper/spa-oauth`.

## Install

```bash
npm install @gatekeeper/angularjs angular
```

## Usage

```js
import angular from 'angular';
import { GATEKEEPER_MODULE } from '@gatekeeper/angularjs';

angular
  .module('app', [GATEKEEPER_MODULE])
  .config(['gatekeeperProvider', function (gatekeeperProvider) {
    gatekeeperProvider.configure({
      idpBaseUrl: 'http://localhost:8080',
      authUiBaseUrl: 'http://localhost:5173',
      tenantId: '<tenant-uuid>',
      clientId: '<public-client-id>',
      redirectUri: 'http://localhost:5178/'
    });
  }])
  .controller('AppController', ['gatekeeper', function (gatekeeper) {
    this.gatekeeper = gatekeeper;
    this.signIn = () => gatekeeper.signIn();
    this.signOut = () => gatekeeper.signOut();
  }]);
```

```html
<div ng-controller="AppController as app">
  <button ng-if="!app.gatekeeper.isAuthenticated()" ng-click="app.signIn()">Sign in</button>
  <div ng-if="app.gatekeeper.isAuthenticated()">
    <span>{{ app.gatekeeper.user().email }}</span>
    <button ng-click="app.signOut()">Sign out</button>
  </div>
</div>
```

The service pushes state changes into a digest via `$rootScope.$applyAsync()`, and
finishes the redirect callback automatically the first time the service is injected.

The `gatekeeper` service exposes `isAuthenticated()`, `user()`, `status()`, `error()`,
`expiresAt()`, `signIn()`, `signOut()`, `refresh()`, `getAccessToken()`,
`fetchUserInfo()`, and the raw `client`.

## Guard a route

```js
import { requireAuthenticated } from '@gatekeeper/angularjs';
$routeProvider.when('/account', { templateUrl: 'account.html', resolve: { auth: requireAuthenticated } });
```

## Public client only

Tokens live in the browser (`sessionStorage`). Register with `client_type: "public"`
(no secret) and add the app origin to the IdP's `CORS_ALLOWED_ORIGINS`.
