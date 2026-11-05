import { redirect } from 'next/navigation';
import { gatekeeper } from '@/lib/gatekeeper';

export default async function Protected() {
  const session = await gatekeeper.getSession();
  if (!session) redirect('/auth/sign-in?returnTo=/protected');

  return (
    <main className="app">
      <header>
        <h1>Protected</h1>
        <p className="subtitle">Only visible with a valid session cookie.</p>
      </header>
      <section className="card">
        <pre>{JSON.stringify(session.profile, null, 2)}</pre>
        <div className="actions">
          <a className="button" href="/">Home</a>
        </div>
      </section>
    </main>
  );
}
