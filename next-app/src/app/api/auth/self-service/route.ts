import { NextRequest, NextResponse } from "next/server";

/**
 * POST /api/auth/self-service
 *
 * Receives an access token from a client application via HTML form POST,
 * stores it in cookies on the IDP domain, and redirects to the self-service
 * profile page.
 *
 * This avoids exposing the access token in URL query parameters.
 *
 * Expected form fields:
 * - token: JWT access token
 * - return_url: URL to redirect back to when the user is done
 */

function decodeJwtPayload(token: string): Record<string, unknown> {
  const [, payload] = token.split(".");
  return JSON.parse(Buffer.from(payload, "base64url").toString("utf-8"));
}

export async function POST(request: NextRequest) {
  const formData = await request.formData();

  const token = formData.get("token") as string | null;
  const returnUrl = formData.get("return_url") as string | null;

  if (!token) {
    return NextResponse.json(
      { error: "Missing access token" },
      { status: 400 },
    );
  }

  // Decode the JWT payload to check expiration
  let payload: Record<string, unknown>;

  try {
    payload = decodeJwtPayload(token);
  } catch {
    return NextResponse.json(
      { error: "Invalid token format" },
      { status: 400 },
    );
  }

  // Check if the token is expired
  const exp = payload.exp as number | undefined;
  const now = Math.floor(Date.now() / 1000);

  if (exp && exp <= now) {
    // Token is expired — redirect back to the client with an error
    if (returnUrl) {
      return NextResponse.redirect(
        new URL(`${returnUrl}?error=token_expired`),
        303,
      );
    }

    return NextResponse.json(
      { error: "Access token has expired. Please sign in again." },
      { status: 401 },
    );
  }

  // Compute remaining token lifetime for a precise cookie maxAge
  const remainingSeconds = exp ? exp - now : 60 * 15;

  // Use 303 See Other (POST → GET redirect, correct PRG pattern)
  const response = NextResponse.redirect(
    new URL("/profile", request.nextUrl.origin),
    303,
  );

  // Store the access token in a cookie on the IDP domain.
  // httpOnly: true — only the profile page server component reads this cookie.
  response.cookies.set("gk_self_service_token", token, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    path: "/",
    sameSite: "lax",
    maxAge: remainingSeconds,
  });

  if (returnUrl) {
    response.cookies.set("gk_return_url", returnUrl, {
      httpOnly: true,
      secure: process.env.NODE_ENV === "production",
      path: "/",
      sameSite: "lax",
      maxAge: remainingSeconds,
    });
  }

  return response;
}
