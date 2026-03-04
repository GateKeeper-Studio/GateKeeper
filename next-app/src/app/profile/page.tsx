import { cookies } from "next/headers";
import { redirect } from "next/navigation";

import ProfileWrapper from "./(components)/profile-wrapper";

type JwtPayload = {
  sub: string;
  given_name: string;
  family_name: string;
  name: string;
  email: string;
  app_id: string;
  exp?: number;
};

function decodeJwtPayload(token: string): JwtPayload {
  const [, payload] = token.split(".");
  const decoded = Buffer.from(payload, "base64url").toString("utf-8");
  return JSON.parse(decoded);
}

/**
 * Self-service profile page (server component).
 *
 * Reads the access token from the `gk_self_service_token` cookie
 * (set by POST /api/auth/self-service), decodes JWT claims, and
 * passes everything to the interactive ProfileWrapper client component.
 */
export default async function ProfilePage() {
  const cookieStore = await cookies();

  const tokenCookie = cookieStore.get("gk_self_service_token");
  const returnUrlCookie = cookieStore.get("gk_return_url");

  if (!tokenCookie?.value) {
    redirect("/");
  }

  const accessToken = tokenCookie.value;
  const returnUrl = returnUrlCookie?.value ?? null;

  let claims: JwtPayload;

  try {
    claims = decodeJwtPayload(accessToken);
  } catch {
    // Invalid token format — redirect to home
    redirect("/");
  }

  // Safety check: if the token has expired, redirect back to the client app
  if (claims.exp && claims.exp <= Math.floor(Date.now() / 1000)) {
    if (returnUrl) {
      redirect(`${returnUrl}?error=token_expired`);
    }
    redirect("/");
  }

  return (
    <ProfileWrapper
      accessToken={accessToken}
      userId={claims.sub}
      applicationId={claims.app_id}
      email={claims.email}
      firstName={claims.given_name}
      lastName={claims.family_name}
      displayName={claims.name}
      returnUrl={returnUrl}
    />
  );
}
