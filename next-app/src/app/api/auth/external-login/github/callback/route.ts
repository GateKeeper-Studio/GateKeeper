// /app/api/auth/github/callback/route.ts
import { NextRequest, NextResponse } from "next/server";
import { cookies } from "next/headers";

// Define the shape of the credentials we fetch from our own API
type OAuthProviderResponse = {
  clientId: string;
  clientSecret: string;
};

// Define the shape of the response from GitHub's token endpoint
type GitHubTokenResponse = {
  access_token: string;
  scope: string;
  token_type: string;
};

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;

  const code = searchParams.get("code");
  const state = searchParams.get("state");

  const cookieStore = await cookies();
  const storedState = cookieStore.get("oauth_state")?.value;
  const oauthProviderId = cookieStore.get("oauth_provider_id")?.value;

  // Clean up the temporary cookies immediately
  cookieStore.delete("oauth_state");
  cookieStore.delete("oauth_provider_id");

  // 1. --- SECURITY CHECK: Validate the state parameter ---
  if (!state || !storedState || state !== storedState || !oauthProviderId) {
    return NextResponse.redirect(
      new URL("/login?error=invalid_state", request.url)
    );
  }

  if (!code) {
    return NextResponse.redirect(new URL("/login?error=no_code", request.url));
  }

  try {
    // 2. --- FETCH CREDENTIALS: Get the Client ID and Secret from your backend ---
    const credsResult = await fetch(
      `${process.env.NEXT_PUBLIC_BASE_API_URL}/v1/auth/application/oauth-provider/${oauthProviderId}`
    );

    if (!credsResult.ok) {
      throw new Error("Failed to fetch OAuth provider credentials.");
    }

    const credentials = (await credsResult.json()) as OAuthProviderResponse;
    const { clientId, clientSecret } = credentials;

    // 3. --- SERVER AUTHENTICATION: Exchange the code for an access token ---
    // This is the critical step that uses the client_secret.
    // This request happens server-to-server and is never exposed to the browser.
    const tokenResponse = await fetch(
      "https://github.com/login/oauth/access_token",
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json", // Important to get a JSON response
        },
        body: JSON.stringify({
          client_id: clientId,
          client_secret: clientSecret,
          code: code,
        }),
      }
    );

    if (!tokenResponse.ok) {
      const errorBody = await tokenResponse.json();
      console.error("GitHub token exchange failed:", errorBody);
      throw new Error("Could not get access token from GitHub.");
    }

    const tokenData = (await tokenResponse.json()) as GitHubTokenResponse;
    const accessToken = tokenData.access_token;

    // 4. --- FETCH USER DATA: Use the access token to get user details ---
    const userResponse = await fetch("https://api.github.com/user", {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });

    if (!userResponse.ok) {
      throw new Error("Failed to fetch user data from GitHub.");
    }

    const userData = await userResponse.json();

    // 5. --- CREATE SESSION & REDIRECT ---
    // At this point, you have the user's data (userData).
    // You should now create a session for your application (e.g., using a JWT or a library like iron-session),
    // store the user details, and set a session cookie.

    // For this example, we'll just redirect to a dashboard page.
    console.log("Successfully authenticated user:", userData);

    // After creating your session, redirect the user to a protected page.
    return NextResponse.redirect(new URL("/dashboard", request.url));
  } catch (error) {
    console.error("An error occurred during the OAuth callback:", error);
    // Redirect to a generic error page
    return NextResponse.redirect(
      new URL("/login?error=authentication_failed", request.url)
    );
  }
}
