import { useState } from 'react';
import { SignInButton, SignOutButton, useGateKeeper } from '@gatekeeper/react';

export function App(): JSX.Element {
  const { isAuthenticated, user, expiresAt, error, refresh, getAccessToken, client } = useGateKeeper();
  const [userinfo, setUserinfo] = useState<string>('No userinfo loaded.');

  async function loadUserInfo(): Promise<void> {
    try {
      const claims = await client.fetchUserInfo();
      setUserinfo(JSON.stringify(claims, null, 2));
    } catch (err) {
      setUserinfo(err instanceof Error ? err.message : 'Request failed');
    }
  }

  async function showToken(): Promise<void> {
    const token = await getAccessToken();
    setUserinfo(token ? `access_token (truncated): ${token.slice(0, 32)}…` : 'No access token.');
  }

  return (
    <main className="app">
      <header>
        <h1>GateKeeper · React public client</h1>
        <p className="subtitle">Authorization Code + PKCE, no client secret.</p>
      </header>

      <section className="card">
        <div className="status">
          <span className={isAuthenticated ? 'dot on' : 'dot off'} />
          <strong>{isAuthenticated ? 'Signed in' : 'Signed out'}</strong>
        </div>

        <dl>
          <dt>User</dt>
          <dd>{user?.email || user?.sub || '—'}</dd>
          <dt>Expires</dt>
          <dd>{expiresAt ? new Date(expiresAt * 1000).toLocaleString() : '—'}</dd>
        </dl>

        {error && <p className="error">{error.message}</p>}

        <div className="actions">
          {isAuthenticated ? (
            <>
              <button type="button" onClick={() => void refresh()}>Refresh token</button>
              <button type="button" onClick={() => void loadUserInfo()}>Load userinfo</button>
              <button type="button" onClick={() => void showToken()}>Show access token</button>
              <SignOutButton />
            </>
          ) : (
            <SignInButton>Sign in with GateKeeper</SignInButton>
          )}
        </div>
      </section>

      <section className="card">
        <h2>Output</h2>
        <pre>{userinfo}</pre>
      </section>
    </main>
  );
}
