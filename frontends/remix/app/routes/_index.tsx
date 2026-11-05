import { json, type LoaderFunctionArgs } from '@remix-run/node';
import { Form, useLoaderData } from '@remix-run/react';
import { gatekeeper } from '~/gatekeeper.server';

export async function loader({ request }: LoaderFunctionArgs) {
  const session = gatekeeper.getSession(request);
  return json({ user: session?.profile ?? null, expiresAt: session?.expiresAt ?? null });
}

export default function Index() {
  const { user, expiresAt } = useLoaderData<typeof loader>();

  return (
    <main className="app">
      <header>
        <h1>GateKeeper · Remix BFF</h1>
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
              <dd>{expiresAt ? new Date(expiresAt * 1000).toLocaleString() : '—'}</dd>
            </dl>
            <div className="actions">
              <a className="button" href="/protected">Protected page</a>
              <Form method="post" action="/auth/sign-out">
                <button type="submit">Sign out</button>
              </Form>
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
