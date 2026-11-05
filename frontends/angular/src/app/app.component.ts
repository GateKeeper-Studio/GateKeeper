import { Component, inject, signal } from '@angular/core';
import { GateKeeperService } from '@gatekeeper/angular';

@Component({
  selector: 'app-root',
  standalone: true,
  template: `
    <main class="app">
      <header>
        <h1>GateKeeper · Angular public client</h1>
        <p class="subtitle">Authorization Code + PKCE, no client secret.</p>
      </header>

      <section class="card">
        <div class="status">
          <span class="dot" [class.on]="gk.isAuthenticated()" [class.off]="!gk.isAuthenticated()"></span>
          <strong>{{ gk.isAuthenticated() ? 'Signed in' : 'Signed out' }}</strong>
        </div>

        <dl>
          <dt>User</dt>
          <dd>{{ gk.user()?.email || gk.user()?.sub || '—' }}</dd>
          <dt>Expires</dt>
          <dd>{{ gk.expiresAt() ? formatExpiry(gk.expiresAt()!) : '—' }}</dd>
        </dl>

        @if (gk.error()) {
          <p class="error">{{ gk.error()?.message }}</p>
        }

        <div class="actions">
          @if (gk.isAuthenticated()) {
            <button type="button" (click)="gk.refresh()">Refresh token</button>
            <button type="button" (click)="loadUserInfo()">Load userinfo</button>
            <button type="button" (click)="showToken()">Show access token</button>
            <button type="button" (click)="gk.signOut()">Sign out</button>
          } @else {
            <button type="button" (click)="gk.signIn()">Sign in with GateKeeper</button>
          }
        </div>
      </section>

      <section class="card">
        <h2>Output</h2>
        <pre>{{ output() }}</pre>
      </section>
    </main>
  `
})
export class AppComponent {
  readonly gk = inject(GateKeeperService);
  readonly output = signal('No userinfo loaded.');

  formatExpiry(seconds: number): string {
    return new Date(seconds * 1000).toLocaleString();
  }

  async loadUserInfo(): Promise<void> {
    try {
      this.output.set(JSON.stringify(await this.gk.fetchUserInfo(), null, 2));
    } catch (err) {
      this.output.set(err instanceof Error ? err.message : 'Request failed');
    }
  }

  async showToken(): Promise<void> {
    const token = await this.gk.getAccessToken();
    this.output.set(token ? `access_token (truncated): ${token.slice(0, 32)}…` : 'No access token.');
  }
}
