type Response = {
  sessionId: string;
  options: unknown;
};

export async function beginWebAuthnRegistration(): Promise<Response> {
  const sessionResponse = await fetch("/api/session");
  const sessionData = (await sessionResponse.json()) as GateKeeperSession;

  const response = await fetch(
    "http://localhost:8080/v1/auth/webauthn/begin-registration",
    {
      method: "POST",
      body: JSON.stringify({
        userId: sessionData.user.id,
        applicationId: sessionData.user.applicationId,
      }),
      headers: {
        "Content-Type": "application/json",
      },
    },
  );

  if (!response.ok) {
    const errorData = (await response.json()) as { message?: string };

    throw new Error(
      errorData.message || "Failed to begin WebAuthn registration",
    );
  }

  const data = (await response.json()) as Response;

  return data;
}
