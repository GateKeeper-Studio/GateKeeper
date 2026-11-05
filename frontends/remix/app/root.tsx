import { json, type LinksFunction, type LoaderFunctionArgs } from '@remix-run/node';
import { Links, Meta, Outlet, Scripts, ScrollRestoration } from '@remix-run/react';
import { gatekeeper } from '~/gatekeeper.server';
import styles from './styles.css?url';

export const links: LinksFunction = () => [{ rel: 'stylesheet', href: styles }];

// Root loader: silent, coalesced refresh. Merge the returned Set-Cookie headers so
// the rotated session cookie is written back to the browser.
export async function loader({ request }: LoaderFunctionArgs) {
  const { session, headers } = await gatekeeper.loadSession(request);
  return json({ user: session?.profile ?? null }, { headers });
}

export default function App() {
  return (
    <html lang="en">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <Meta />
        <Links />
      </head>
      <body>
        <Outlet />
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}
