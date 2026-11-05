import {
  APP_INITIALIZER,
  makeEnvironmentProviders,
  type EnvironmentProviders
} from '@angular/core';
import type { GateKeeperSpaConfig } from '@gatekeeper/spa-oauth';
import { GATEKEEPER_CONFIG, GateKeeperService } from './gatekeeper.service';

/**
 * Register GateKeeper in a standalone Angular app:
 *
 * ```ts
 * bootstrapApplication(AppComponent, {
 *   providers: [provideGateKeeper({ idpBaseUrl, authUiBaseUrl, tenantId, clientId, redirectUri })]
 * });
 * ```
 *
 * Also runs `handleRedirectCallback()` at startup so the flow finishes after the
 * redirect back from the hosted login UI.
 */
export function provideGateKeeper(config: GateKeeperSpaConfig): EnvironmentProviders {
  return makeEnvironmentProviders([
    { provide: GATEKEEPER_CONFIG, useValue: config },
    {
      provide: APP_INITIALIZER,
      multi: true,
      deps: [GateKeeperService],
      useFactory: (gatekeeper: GateKeeperService) => () =>
        gatekeeper.handleRedirectCallback().catch(() => {
          /* surfaced via GateKeeperService.error() */
        })
    }
  ]);
}
