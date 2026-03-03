"use client";

import { toast } from "sonner";
import { useState, useEffect, useCallback } from "react";
import { useParams, useRouter, useSearchParams } from "next/navigation";
import { startAuthentication } from "@simplewebauthn/browser";

import { Button } from "@/components/ui/button";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import { ErrorAlert } from "@/components/error-alert";

import { authorizeApi } from "@/services/auth/authorize";
import { verifyWebAuthnMfaApi } from "@/services/auth/verify-webauthn-mfa";

export function AuthForm() {
  const applicationId = useParams().applicationId as string;
  const searchParams = useSearchParams();
  const router = useRouter();

  const redirectUri = searchParams.get("redirect_uri") || "/";
  const codeChallengeMethod = searchParams.get("code_challenge_method") || "";
  const responseType = searchParams.get("response_type") || "";
  const scope = searchParams.get("scope") || "";
  const state = searchParams.get("state") || "";
  const email = searchParams.get("email") || "";
  const codeChallenge = searchParams.get("code_challenge") || "";
  const mfaId = searchParams.get("mfa_id") || "";
  const nonce = searchParams.get("nonce") || "";

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleWebAuthn = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    const storedOptions = sessionStorage.getItem(`webauthn_options_${mfaId}`);

    if (!storedOptions) {
      setError("WebAuthn options not found. Please sign in again.");
      setIsLoading(false);
      return;
    }

    let assertionOptions;
    try {
      const parsed = JSON.parse(storedOptions);
      assertionOptions = parsed?.publicKey ?? parsed;
    } catch (err) {
      console.error("Failed to parse WebAuthn options:", storedOptions, err);
      setError("Invalid WebAuthn options. Please sign in again.");
      setIsLoading(false);
      return;
    }

    let assertionResponse;
    try {
      assertionResponse = await startAuthentication({
        optionsJSON: assertionOptions,
      });
    } catch (err: unknown) {
      console.error("WebAuthn authentication error:", err);

      const message =
        err instanceof Error
          ? err.message
          : "Passkey authentication was cancelled or failed.";
      setError(message);
      setIsLoading(false);
      return;
    }

    const [verifyData, verifyErr] = await verifyWebAuthnMfaApi({
      email: email.trim(),
      sessionId: mfaId,
      assertionData: assertionResponse,
      applicationId,
    });

    if (verifyErr) {
      console.error("WebAuthn verification error:", verifyErr);
      setError(
        verifyErr?.response?.data.message || "Passkey verification failed.",
      );
      setIsLoading(false);
      return;
    }

    if (!verifyData) {
      setError("An error occurred during verification.");
      setIsLoading(false);
      return;
    }

    sessionStorage.removeItem(`webauthn_options_${mfaId}`);

    const [authorizeData, authorizeErr] = await authorizeApi({
      email: email.trim(),
      sessionCode: verifyData.sessionCode,
      applicationId,
      redirectUri,
      responseType,
      scope,
      codeChallengeMethod,
      codeChallenge,
      state,
      nonce: nonce || undefined,
    });

    if (authorizeErr) {
      setError(authorizeErr?.response?.data.message || "Authorization failed.");
      setIsLoading(false);
      return;
    }

    if (!authorizeData) {
      setError("An error occurred during authorization.");
      setIsLoading(false);
      return;
    }

    setIsLoading(false);
    toast.success("You have successfully signed in");

    window.location.href = `${redirectUri}?code=${authorizeData.authorizationCode}&state=${state}&redirect_uri=${redirectUri}&client_id=${applicationId}`;
  }, [
    applicationId,
    codeChallenge,
    codeChallengeMethod,
    email,
    mfaId,
    nonce,
    redirectUri,
    responseType,
    scope,
    state,
  ]);

  useEffect(() => {
    if (mfaId) {
      handleWebAuthn();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [handleWebAuthn]);

  function handleSignInAgain() {
    const urlParams = new URLSearchParams({
      redirect_uri: redirectUri,
      response_type: responseType,
      scope,
      code_challenge_method: codeChallengeMethod,
      code_challenge: codeChallenge,
      state,
      ...(nonce ? { nonce } : {}),
    });
    router.push(`/auth/${applicationId}/sign-in?${urlParams.toString()}`);
  }

  return (
    <div className="grid gap-4">
      {error && <ErrorAlert message={error} title="An error occurred..." />}

      <div className="flex flex-col items-center gap-3">
        {isLoading ? (
          <div className="flex items-center gap-2 text-muted-foreground">
            <LoadingSpinner />
            <span>Waiting for passkey...</span>
          </div>
        ) : (
          <>
            {!error && (
              <p className="text-sm text-muted-foreground text-center">
                Your passkey prompt should appear automatically.
              </p>
            )}
            <Button
              type="button"
              disabled={isLoading}
              className="w-full relative"
              onClick={handleWebAuthn}
            >
              {isLoading && <LoadingSpinner className="absolute left-4" />}
              Use Passkey
            </Button>
            <Button
              type="button"
              variant="ghost"
              className="w-full"
              onClick={handleSignInAgain}
            >
              Sign in again
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
