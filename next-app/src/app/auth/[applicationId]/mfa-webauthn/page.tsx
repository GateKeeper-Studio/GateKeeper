import { AuthForm } from "./(components)/auth-form";
import { Background } from "../(components)/background";
import { ErrorAlert } from "@/components/error-alert";

import { getApplicationAuthDataService } from "@/services/auth/get-application-auth-data";

type Props = {
  params: Promise<{ applicationId: string }>;
};

export default async function MfaWebAuthnPage({ params }: Props) {
  const { applicationId } = await params;

  const [application, err] = await getApplicationAuthDataService({
    applicationId,
  });

  console.log("Application data:", application, err);

  return (
    <Background application={application} page="one-time-password">
      <div className="flex flex-col space-y-2 text-center">
        <h1 className="text-2xl font-semibold tracking-tight">
          Passkey Authentication
        </h1>
        <p className="text-muted-foreground text-sm">
          Use your passkey or security key to verify your identity
        </p>
      </div>

      {err ? (
        <ErrorAlert message={err.message} title="An error occurred..." />
      ) : (
        <AuthForm />
      )}
    </Background>
  );
}
