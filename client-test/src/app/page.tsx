import { LogoutButton } from "@/components/logout-button";
import { SignInButton } from "@/components/sign-in-button";
import { GenerateMfaSecretButton } from "@/components/generate-mfa-secret-button";
import { RegisterPasskeyButton } from "@/components/register-passkey-button";

import { getServerSession } from "@/lib/utils/get-server-session";

export default async function Home() {
  const [session] = await getServerSession();

  return (
    <div className="grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-8 pb-20 gap-16 sm:p-20 font-[family-name:var(--font-geist-sans)]">
      <main className="flex flex-col gap-8 row-start-2 items-start">
        <SignInButton />
        <LogoutButton />

        {session && (
          <>
            <pre>
              <code>{JSON.stringify(session, null, 2)}</code>
            </pre>

            <GenerateMfaSecretButton />
            <RegisterPasskeyButton />
          </>
        )}
      </main>
    </div>
  );
}
