type Response = {
  otpUrl: string;
};

type Props = {
  mfaAuthAppCode: string;
};

export async function confirmMfaUserSecret({
  mfaAuthAppCode,
}: Props): Promise<Response> {
  const sessionResponse = await fetch("/api/session");
  const sessionData = (await sessionResponse.json()) as GateKeeperSession;

  const response = await fetch(
    "http://localhost:8080/v1/auth/confirm-mfa-auth-app-secret",
    {
      method: "POST",
      body: JSON.stringify({
        userId: sessionData.user.id,
        mfaAuthAppCode,
      }),
      headers: {
        "Content-Type": "application/json",
      },
    }
  );

  const data = (await response.json()) as Response;

  if (!response.ok) {
    console.error("Error confirming MFA secret:", data);

    throw new Error(
      (data as unknown as { message: string }).message ||
        "Failed to confirm MFA secret"
    );
  }

  return data;
}
