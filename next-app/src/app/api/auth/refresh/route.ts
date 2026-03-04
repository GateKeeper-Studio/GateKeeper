import { cookies } from "next/headers";
import { NextResponse } from "next/server";

const GATEKEEPER_API_URL =
  process.env.NEXT_PUBLIC_BASE_API_URL || "http://localhost:8080";

function decodeJwtPayload(token: string): Record<string, unknown> {
  const [, payload] = token.split(".");
  return JSON.parse(Buffer.from(payload, "base64url").toString("utf-8"));
}

/**
 * POST /api/auth/refresh
 *
 * Reads the current `gk_self_service_token` cookie, calls the Go server
 * refresh endpoint (which accepts recently-expired tokens), and stores
 * the new token in the cookie. Returns the new token + claims to the client.
 */
export async function POST() {
  const cookieStore = await cookies();
  const tokenCookie = cookieStore.get("gk_self_service_token");

  if (!tokenCookie?.value) {
    return NextResponse.json(
      { error: "No session token found" },
      { status: 401 },
    );
  }

  const currentToken = tokenCookie.value;

  // Call the Go server refresh endpoint
  const response = await fetch(`${GATEKEEPER_API_URL}/v1/account/refresh`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${currentToken}`,
      "Content-Type": "application/json",
    },
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({}));
    return NextResponse.json(
      { error: errorData.message || "Failed to refresh token" },
      { status: response.status },
    );
  }

  const data = (await response.json()) as {
    accessToken: string;
    expiresIn: number;
  };

  // Decode the new token to extract claims
  let claims: Record<string, unknown>;
  try {
    claims = decodeJwtPayload(data.accessToken);
  } catch {
    return NextResponse.json(
      { error: "Invalid token received from server" },
      { status: 500 },
    );
  }

  // Update the cookie with the new token
  const res = NextResponse.json({
    accessToken: data.accessToken,
    expiresIn: data.expiresIn,
    claims: {
      sub: claims.sub,
      given_name: claims.given_name,
      family_name: claims.family_name,
      name: claims.name,
      email: claims.email,
      app_id: claims.app_id,
      exp: claims.exp,
    },
  });

  res.cookies.set("gk_self_service_token", data.accessToken, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    path: "/",
    sameSite: "lax",
    maxAge: data.expiresIn,
  });

  return res;
}
