// /app/api/auth/github/login/route.ts
import { NextRequest, NextResponse } from "next/server";
import { cookies } from "next/headers";

type RequestBody = {
  oauthProviderId: string;
};

export type OAuthProviderResponse = {
  id: string;
  name: string;
  enabled: boolean;
  applicationId: string;
  createdAt: string;
  updatedAt: string | null;
  clientId: string;
  clientSecret: string;
  redirectUri: string;
};

export async function POST(request: NextRequest) {
  const requestBody = (await request.json()) as RequestBody;
  const { oauthProviderId } = requestBody; // Destructure for clarity

  const state = crypto.randomUUID();
  const scope = "read:user user:email";

  const result = await fetch(
    `${process.env.NEXT_PUBLIC_BASE_API_URL}/v1/auth/application/oauth-provider/${oauthProviderId}`
  );

  // Add error handling for your API call
  if (!result.ok) {
    console.error(
      "Failed to fetch OAuth provider credentials:",
      await result.text()
    );
    return NextResponse.json(
      { error: "Could not retrieve OAuth provider details." },
      { status: 500 }
    );
  }

  const credentials = (await result.json()) as OAuthProviderResponse;
  const githubClientID = credentials.clientId;
  const redirectUri = credentials.redirectUri;

  const params = new URLSearchParams({
    client_id: githubClientID,
    redirect_uri: redirectUri,
    scope: scope,
    state: state,
  });

  const cookieStore = await cookies();

  // Store the state in a temporary, httpOnly cookie
  cookieStore.set("oauth_state", state, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    maxAge: 60 * 10, // 10 minutes
    path: "/",
  });

  // *** ADDITION: Store the provider ID to use in the callback ***
  cookieStore.set("oauth_provider_id", oauthProviderId, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    maxAge: 60 * 10, // 10 minutes
    path: "/",
  });

  const authUrl = `https://github.com/login/oauth/authorize?${params.toString()}`;

  return NextResponse.json({ url: authUrl });
}
