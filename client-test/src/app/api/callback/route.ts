import { NextRequest, NextResponse } from "next/server";
import { hashSessionObjectWithPassword } from "@/lib/utils/hash-session";
import { generateCodeChallenge } from "../sign-in/route";

type TokenResponse = GateKeeperSession & {
  idToken?: string;
};

/** Decode a JWT payload without verifying the signature (client-side nonce check only). */
function decodeJwtPayload(token: string): Record<string, unknown> | null {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return null;
    const payload = Buffer.from(parts[1], "base64url").toString("utf-8");
    return JSON.parse(payload) as Record<string, unknown>;
  } catch {
    return null;
  }
}

export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const code = searchParams.get("code") ?? "";
  const state = searchParams.get("state");
  const clientId = searchParams.get("client_id") ?? "";
  const redirectUri = searchParams.get("redirect_uri");

  // Recupera os cookies armazenados na rota de login
  const stateCookie = request.cookies.get("gk_state");
  const codeVerifierCookie = request.cookies.get("gk_code_verifier");
  const nonceCookie = request.cookies.get("gk_nonce");

  // Validação do state para proteção contra CSRF
  if (!state || !code || state !== stateCookie?.value) {
    return new NextResponse("Invalid State", { status: 400 });
  }

  // Prepara a requisição para trocar o código pelo token
  const tokenEndpoint = `${process.env.GATEKEEPER_SERVICE_URL}/v1/auth/sign-in`;
  const responseData = await fetch(tokenEndpoint, {
    method: "POST",
    headers: {
      "Content-Type": "application/x-www-form-urlencoded",
    },
    body: JSON.stringify({
      grantType: "authorization_code",
      authorizationCode: code,
      redirectUri: redirectUri ?? "/",
      clientId: clientId,
      clientSecret: process.env.GATEKEEPER_CLIENT_SECRET || "",
      codeVerifier: codeVerifierCookie?.value || "",
    }),
  });

  const data = (await responseData.json()) as TokenResponse;

  if (responseData.ok === false) {
    return new NextResponse("Invalid Code", { status: 400 });
  }

  // Validate nonce from ID Token to prevent replay attacks (OIDC spec requirement)
  if (data.idToken && nonceCookie?.value) {
    const idTokenPayload = decodeJwtPayload(data.idToken);
    const idTokenNonce = idTokenPayload?.["nonce"] as string | undefined;
    if (idTokenNonce !== nonceCookie.value) {
      return new NextResponse("Nonce mismatch – possible replay attack", {
        status: 400,
      });
    }
  }

  const response = NextResponse.redirect("http://localhost:3001");

  const [encryptedSession, err] = await hashSessionObjectWithPassword(
    data,
    process.env.SESSION_SECRET || "",
  );

  if (err) {
    console.error("Error hashing session:", err);
    return new NextResponse("Internal Server Error", { status: 500 });
  }

  response.cookies.set("gk_session", encryptedSession || "", {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
  });

  response.cookies.delete("gk_state");
  response.cookies.delete("gk_code_verifier");
  response.cookies.delete("gk_nonce");

  return response;
}
