type Props = {
  sessionId: string;
  credentialData: unknown;
};

export async function verifyWebAuthnRegistration({
  sessionId,
  credentialData,
}: Props): Promise<void> {
  const sessionResponse = await fetch("/api/session");
  const sessionData = (await sessionResponse.json()) as GateKeeperSession;

  const response = await fetch(
    "http://localhost:8080/v1/auth/webauthn/verify-registration",
    {
      method: "POST",
      body: JSON.stringify({
        userId: sessionData.user.id,
        applicationId: sessionData.user.applicationId,
        sessionId,
        credentialData,
      }),
      headers: {
        "Content-Type": "application/json",
      },
    },
  );

  if (!response.ok) {
    const errorData = (await response.json()) as { message?: string };

    throw new Error(
      errorData.message || "Failed to verify WebAuthn registration",
    );
  }
}
