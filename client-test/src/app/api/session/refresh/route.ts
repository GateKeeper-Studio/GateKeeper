import { cookies } from "next/headers";
import { getServerSession } from "@/lib/utils/get-server-session";
import { hashSessionObjectWithPassword } from "@/lib/utils/hash-session";

export async function POST() {
  const [session, err] = await getServerSession();

  if (err || !session) {
    return new Response(JSON.stringify({ message: "Unauthorized" }), {
      status: 401,
    });
  }

  const serviceUrl = process.env.GATEKEEPER_SERVICE_URL;
  const sessionSecret = process.env.SESSION_SECRET;

  if (!serviceUrl || !sessionSecret) {
    return new Response(
      JSON.stringify({ message: "Server misconfiguration" }),
      { status: 500 },
    );
  }

  // Call the GateKeeper refresh endpoint
  const refreshRes = await fetch(`${serviceUrl}/v1/account/refresh`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${session.accessToken}`,
      "Content-Type": "application/json",
    },
  });

  if (!refreshRes.ok) {
    return new Response(
      JSON.stringify({ message: "Failed to refresh token" }),
      { status: refreshRes.status },
    );
  }

  const { accessToken, expiresIn } = (await refreshRes.json()) as {
    accessToken: string;
    expiresIn: number;
  };

  // Update the session with the new access token
  const updatedSession: GateKeeperSession = {
    ...session,
    accessToken,
  };

  const [encrypted, hashErr] = await hashSessionObjectWithPassword(
    updatedSession as unknown as Record<string, unknown>,
    sessionSecret,
  );

  if (hashErr || !encrypted) {
    return new Response(
      JSON.stringify({ message: "Failed to encrypt session" }),
      { status: 500 },
    );
  }

  // Update the session cookie
  const cookieStore = await cookies();
  cookieStore.set("gk_session", encrypted, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    maxAge: expiresIn,
  });

  return new Response(
    JSON.stringify({
      session: updatedSession,
      expiresIn,
    }),
    {
      headers: { "Content-Type": "application/json" },
    },
  );
}
