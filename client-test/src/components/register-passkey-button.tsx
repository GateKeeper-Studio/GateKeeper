"use client";

import { useState } from "react";
import { startRegistration } from "@simplewebauthn/browser";

import { beginWebAuthnRegistration } from "@/lib/utils/begin-webauthn-registration";
import { verifyWebAuthnRegistration } from "@/lib/utils/verify-webauthn-registration";

export function RegisterPasskeyButton() {
  const [isRegistering, setIsRegistering] = useState(false);
  const [status, setStatus] = useState<
    "idle" | "success" | "error"
  >("idle");

  async function handleRegisterPasskey() {
    setIsRegistering(true);
    setStatus("idle");

    try {
      // Step 1: Begin registration — get options from the server
      const beginData = await beginWebAuthnRegistration();

      // Step 2: Start the browser WebAuthn ceremony
      const rawOptions = beginData.options as { publicKey?: unknown } | null;
      const registrationOptions = rawOptions?.publicKey ?? rawOptions;

      const registrationResponse = await startRegistration({
        optionsJSON: registrationOptions as Parameters<
          typeof startRegistration
        >[0]["optionsJSON"],
      });

      // Step 3: Verify the registration with the server
      await verifyWebAuthnRegistration({
        sessionId: beginData.sessionId,
        credentialData: registrationResponse,
      });

      setStatus("success");
      alert("Passkey registered successfully!");
    } catch (error) {
      setStatus("error");
      const message =
        error instanceof Error
          ? error.message
          : "Passkey registration failed.";

      console.error("Passkey registration error:", error);
      alert(message);
    } finally {
      setIsRegistering(false);
    }
  }

  return (
    <div className="flex flex-col items-center gap-3 p-4 bg-white rounded-md shadow-md">
      <button
        onClick={handleRegisterPasskey}
        disabled={isRegistering}
        type="button"
        className="px-4 py-2 text-white bg-purple-600 rounded-md shadow-md hover:bg-purple-700 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 focus:ring-offset-white disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {isRegistering ? "Registering Passkey..." : "Register Passkey (WebAuthn)"}
      </button>

      {status === "success" && (
        <span className="text-sm text-green-600 font-medium">
          ✓ Passkey configured
        </span>
      )}

      {status === "error" && (
        <span className="text-sm text-red-600 font-medium">
          ✗ Registration failed
        </span>
      )}
    </div>
  );
}
