import { json, redirect, type LoaderFunctionArgs } from '@remix-run/node';
import { useLoaderData } from '@remix-run/react';
import { gatekeeper } from '~/gatekeeper.server';

export async function loader({ request }: LoaderFunctionArgs) {
  const session = gatekeeper.getSession(request);
  if (!session) throw redirect('/auth/sign-in?returnTo=/protected');
  return json({ profile: session.profile });
}

export default function Protected() {
  const { profile } = useLoaderData<typeof loader>();
  return (
    <main className="app">
      <header>
        <h1>Protected</h1>
        <p className="subtitle">Only visible with a valid session cookie.</p>
      </header>
      <section className="card">
        <pre>{JSON.stringify(profile, null, 2)}</pre>
        <div className="actions">
          <a className="button" href="/">Home</a>
        </div>
      </section>
    </main>
  );
}
