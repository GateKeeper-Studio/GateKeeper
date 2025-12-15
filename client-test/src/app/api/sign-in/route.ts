import { NextRequest, NextResponse } from "next/server";
import { createHash, randomBytes } from "node:crypto";

export async function POST(request: NextRequest) {
  const { redirectUri } = (await request.json()) as { redirectUri: string };

  const clientId = process.env.GATEKEEPER_CLIENT_ID || "";

  const state = randomBytes(16).toString("hex");

  // Se estiver utilizando PKCE, gere o code_verifier e o code_challenge aqui
  const codeVerifier = randomBytes(32).toString("hex");
  const codeChallenge = generateCodeChallenge(codeVerifier);

  const params = new URLSearchParams({
    redirect_uri: redirectUri,
    response_type: "code",
    scope: "openid profile email",
    code_challenge: codeChallenge,
    code_challenge_method: "S256",
    state,
  });

  // Endpoint do Identity Provider que realiza a autorização
  const authorizationEndpoint = `http://localhost:3000/auth/${clientId}/sign-in?${params.toString()}`;
  const response = NextResponse.json({ url: authorizationEndpoint });

  response.cookies.set("gk_code_verifier", codeVerifier, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    path: "/",
    sameSite: "strict",
  });

  response.cookies.set("gk_state", state, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    path: "/",
    sameSite: "strict",
  });

  return response;
}

function generateCodeChallenge(codeVerifier: string): string {
  const hash = createHash("sha256");
  hash.update(codeVerifier);

  function base64UrlEncode(buffer: Buffer): string {
    return buffer
      .toString("base64")
      .replace(/\+/g, "-")
      .replace(/\//g, "_")
      .replace(/=/g, "");
  }

  return base64UrlEncode(hash.digest());
}
