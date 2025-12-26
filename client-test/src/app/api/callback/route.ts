import { NextRequest, NextResponse } from "next/server";
import { hashSessionObjectWithPassword } from "@/lib/utils/hash-session";

export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const code = searchParams.get("code") ?? "";
  const state = searchParams.get("state");
  const clientId = searchParams.get("client_id") ?? "";
  const redirectUri = searchParams.get("redirect_uri");

  console.log({ cookies: request.cookies.getAll() });

  // Recupera os cookies armazenados na rota de login
  const stateCookie = request.cookies.get("gk_state");
  const codeVerifierCookie = request.cookies.get("gk_code_verifier");

  // Validação do state para proteção contra CSRF
  if (!state || !code || state !== stateCookie?.value) {
    return new NextResponse("Invalid State", { status: 400 });
  }

  // Prepara a requisição para trocar o código pelo token
  const tokenEndpoint = "http://localhost:8080/v1/auth/sign-in";
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
      clientSecret: process.env.GATEKEEPER_CLIENT_SECRET || "", // inclua se necessário
      codeVerifier: codeVerifierCookie?.value || "", // inclua se usar PKCE
    }),
  });

  const data = (await responseData.json()) as GateKeeperSession;

  if (responseData.ok === false) {
    return new NextResponse("Invalid Code", { status: 400 });
  }

  const response = NextResponse.redirect("http://localhost:3001/auth/callback");

  const [encryptedSession, err] = await hashSessionObjectWithPassword(
    data,
    process.env.SESSION_SECRET || ""
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

  return response;
}
