import { LogoutButton } from "@/components/logout-button";
import { SignInButton } from "@/components/sign-in-button";
import { GenerateMfaSecretButton } from "@/components/generate-mfa-secret-button";
import { RegisterPasskeyButton } from "@/components/register-passkey-button";

import { getServerSession } from "@/lib/utils/get-server-session";

const IDP_URL = process.env.GATEKEEPER_IDP_URL || "http://localhost:3000";

export default async function Home() {
  const [session] = await getServerSession();

  return (
    <div className="grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-8 pb-20 gap-16 sm:p-20 font-[family-name:var(--font-geist-sans)]">
      <main className="flex flex-col gap-8 row-start-2 items-start">
        <SignInButton />
        <LogoutButton />

        {session && (
          <>
            {/* POST form submits the access token to the IDP self-service portal */}
            <form action={`${IDP_URL}/api/auth/self-service`} method="POST">
              <input type="hidden" name="token" value={session.accessToken} />
              <input
                type="hidden"
                name="return_url"
                value={
                  process.env.NEXT_PUBLIC_APP_URL || "http://localhost:3001"
                }
              />

              <button
                type="submit"
                className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-md hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2"
              >
                My Account
              </button>
            </form>

            <pre>
              <code>{JSON.stringify(session, null, 2)}</code>
            </pre>
          </>
        )}
      </main>
    </div>
  );
}
