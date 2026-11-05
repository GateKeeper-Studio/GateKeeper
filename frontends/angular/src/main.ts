import { bootstrapApplication } from '@angular/platform-browser';
import { provideRouter } from '@angular/router';
import { provideGateKeeper } from '@gatekeeper/angular';
import { AppComponent } from './app/app.component';
import { gatekeeperConfig } from './gatekeeper.config';

bootstrapApplication(AppComponent, {
  providers: [provideRouter([]), provideGateKeeper(gatekeeperConfig)]
}).catch((err) => console.error(err));
