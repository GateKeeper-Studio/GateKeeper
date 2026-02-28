type Response = {
  otpUrl: string;
};

export async function generateMfaToken(): Promise<Response> {
  const sessionResponse = await fetch("/api/session");
  const sessionData = (await sessionResponse.json()) as GateKeeperSession;

  const response = await fetch(
    "http://192.168.0.140:8080/v1/auth/generate-auth-secret",
    {
      method: "POST",
      body: JSON.stringify({
        userId: sessionData.user.id,
        applicationId: sessionData.user.applicationId,
      }),
      headers: {
        "Content-Type": "application/json",
      },
    }
  );

  const data = (await response.json()) as Response;

  return data;
}
