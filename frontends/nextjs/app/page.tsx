import { gatekeeper } from '@/lib/gatekeeper';

export default async function Home() {
  const session = await gatekeeper.getSession();
  const user = session?.profile ?? null;

  return (
    <main className="app">
      <header>
        <h1>GateKeeper · Next.js BFF</h1>
        <p className="subtitle">Confidential client — tokens never reach the browser.</p>
      </header>

      <section className="card">
        <div className="status">
          <span className={user ? 'dot on' : 'dot off'} />
          <strong>{user ? 'Signed in' : 'Signed out'}</strong>
        </div>

        {user ? (
          <>
            <dl>
              <dt>User</dt>
              <dd>{user.email || user.sub || '—'}</dd>
              <dt>Expires</dt>
              <dd>{session ? new Date(session.expiresAt * 1000).toLocaleString() : '—'}</dd>
            </dl>
            <div className="actions">
              <a className="button" href="/protected">Protected page</a>
              <form action="/auth/sign-out" method="post">
                <button type="submit">Sign out</button>
              </form>
            </div>
          </>
        ) : (
          <div className="actions">
            <a className="button" href="/auth/sign-in?returnTo=/">Sign in with GateKeeper</a>
          </div>
        )}
      </section>
    </main>
  );
}
