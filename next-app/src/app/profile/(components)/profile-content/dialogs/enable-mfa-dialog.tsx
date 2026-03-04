"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import {
  Loader2,
  Smartphone,
  ShieldCheck,
  Copy,
  Mail,
  Fingerprint,
} from "lucide-react";
import { toast } from "sonner";
import qrCode from "qrcode";
import { startRegistration } from "@simplewebauthn/browser";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  InputOTP,
  InputOTPGroup,
  InputOTPSlot,
  InputOTPSeparator,
} from "@/components/ui/input-otp";
import { Alert, AlertDescription } from "@/components/ui/alert";

import { generateMfaSecretApi } from "@/services/account/generate-mfa-secret";
import { getLastMfaTotpSecretApi } from "@/services/account/get-last-mfa-totp-secret";
import { confirmMfaAuthAppSecretApi } from "@/services/account/confirm-mfa-auth-app-secret";
import { updatePreferredMfaApi } from "@/services/account/update-preferred-mfa";
import { enableEmailMfaApi } from "@/services/account/enable-email-mfa";
import { beginWebAuthnRegistrationApi } from "@/services/auth/begin-webauthn-registration";
import { verifyWebAuthnRegistrationApi } from "@/services/auth/verify-webauthn-registration";
import { useSession } from "../../session-context";
import { copy } from "@/lib/utils";

type EnableMfaDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess: () => void;
  applicationId: string;
  userId: string;
  mfaType: string;
};

type Step = "generate" | "verify" | "done";

const TYPE_LABELS: Record<string, string> = {
  totp: "Authenticator App",
  email: "Email MFA",
  webauthn: "Passkey / Security Key",
};

const TYPE_ICONS: Record<string, typeof Smartphone> = {
  totp: Smartphone,
  email: Mail,
  webauthn: Fingerprint,
};

export function EnableMfaDialog({
  open,
  onOpenChange,
  onSuccess,
  applicationId,
  userId,
  mfaType,
}: EnableMfaDialogProps) {
  const { accessToken } = useSession();
  const [step, setStep] = useState<Step>("generate");
  const [otpUrl, setOtpUrl] = useState<string | null>(null);
  const [totpCode, setTotpCode] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const canvasRef = useRef<HTMLCanvasElement>(null);

  const Icon = TYPE_ICONS[mfaType] || Smartphone;
  const label = TYPE_LABELS[mfaType] || mfaType;

  // Generate QR code when otpUrl is set
  useEffect(() => {
    if (otpUrl && canvasRef.current) {
      qrCode.toCanvas(
        canvasRef.current,
        otpUrl,
        {
          width: 200,
          margin: 2,
          color: { dark: "#000000", light: "#FFFFFF" },
        },
        (err) => {
          if (err) console.error("QR code error:", err);
        },
      );
    }
  }, [otpUrl]);

  // Generate secret on open for TOTP
  useEffect(() => {
    if (open && mfaType === "totp" && step === "generate" && !otpUrl) {
      handleLoadOrGenerateSecret();
    }
  }, [open, mfaType]);

  const handleLoadOrGenerateSecret = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    // Step 1: Try to get the last valid (non-expired) secret
    const [existingData, existingErr] = await getLastMfaTotpSecretApi({
      accessToken,
    });

    if (!existingErr && existingData) {
      // Valid secret found — reuse it
      setOtpUrl(existingData.otpUrl);
      setStep("verify");
      setIsLoading(false);
      return;
    }

    // Step 2: No valid secret — generate a new one
    const [data, err] = await generateMfaSecretApi(
      { applicationId, userId },
      { accessToken },
    );

    setIsLoading(false);

    if (err) {
      if (err.response?.status === 401) {
        setError("Your session has expired. Please go back and sign in again.");
        return;
      }

      setError(err.response?.data?.message || "Failed to generate MFA secret.");
      return;
    }

    if (data) {
      setOtpUrl(data.otpUrl);
      setStep("verify");
    }
  }, [applicationId, userId, accessToken]);

  const handleVerifyTotp = useCallback(async () => {
    if (totpCode.length !== 6) {
      setError("Enter the 6-digit code from your authenticator app");
      return;
    }

    setIsLoading(true);
    setError(null);

    // Step 1: Confirm the TOTP secret using the existing auth endpoint
    const [confirmErr] = await confirmMfaAuthAppSecretApi(
      { userId, mfaAuthAppCode: totpCode },
      { accessToken },
    );

    if (confirmErr) {
      setIsLoading(false);

      if (confirmErr.response?.status === 401) {
        setError("Your session has expired. Please go back and sign in again.");
        return;
      }

      setError(
        confirmErr.response?.data?.message || "Invalid code. Please try again.",
      );
      return;
    }

    // Step 2: Set the preferred method to TOTP
    const [, prefErr] = await updatePreferredMfaApi(
      { preferredMethod: "totp" },
      { accessToken },
    );

    setIsLoading(false);

    if (prefErr) {
      // The secret was confirmed but preferred method failed — still count as success
      console.error("Failed to set preferred method:", prefErr);
    }

    toast.success("Authenticator app has been configured", {
      description: "Your account is now protected with an authenticator app.",
    });

    onSuccess();
    handleClose(false);
  }, [totpCode, userId, applicationId, accessToken, onSuccess]);

  const handleClose = (open: boolean) => {
    if (!open) {
      setStep("generate");
      setOtpUrl(null);
      setTotpCode("");
      setError(null);
    }
    onOpenChange(open);
  };

  // Extract secret from OTP URL for manual entry
  const secretKey = otpUrl ? new URL(otpUrl).searchParams.get("secret") : null;

  // ─── Email MFA Flow ─────────────────────────────────────────────
  const handleEnableEmailMfa = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    const [, err] = await enableEmailMfaApi({ accessToken });

    if (err) {
      setIsLoading(false);
      if (err.response?.status === 401) {
        setError("Your session has expired. Please go back and sign in again.");
        return;
      }
      setError(err.response?.data?.message || "Failed to enable email MFA.");
      return;
    }

    // Set email as preferred method
    const [, prefErr] = await updatePreferredMfaApi(
      { preferredMethod: "email" },
      { accessToken },
    );

    setIsLoading(false);

    if (prefErr) {
      console.error("Failed to set preferred method:", prefErr);
    }

    toast.success("Email MFA has been enabled", {
      description:
        "A verification code will be sent to your email when you sign in.",
    });

    onSuccess();
    handleClose(false);
  }, [accessToken, onSuccess]);

  // ─── WebAuthn Flow ────────────────────────────────────────────
  const handleEnableWebAuthn = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    // Step 1: Begin registration
    const [beginData, beginErr] = await beginWebAuthnRegistrationApi({
      userId,
      applicationId,
    });

    if (beginErr || !beginData) {
      setIsLoading(false);
      if (beginErr?.response?.status === 401) {
        setError("Your session has expired. Please go back and sign in again.");
        return;
      }
      setError(
        beginErr?.response?.data?.message ||
          "Failed to start passkey registration.",
      );
      return;
    }

    try {
      // Step 2: Browser WebAuthn ceremony
      const rawOptions = beginData.options as { publicKey?: unknown } | null;
      const registrationOptions = rawOptions?.publicKey ?? rawOptions;

      const registrationResponse = await startRegistration({
        optionsJSON: registrationOptions as Parameters<
          typeof startRegistration
        >[0]["optionsJSON"],
      });

      // Step 3: Verify with server
      const [, verifyErr] = await verifyWebAuthnRegistrationApi({
        userId,
        applicationId,
        sessionId: beginData.sessionId,
        credentialData: registrationResponse,
      });

      if (verifyErr) {
        setIsLoading(false);
        setError(
          verifyErr.response?.data?.message ||
            "Failed to verify passkey registration.",
        );
        return;
      }

      // Step 4: Set as preferred
      const [, prefErr] = await updatePreferredMfaApi(
        { preferredMethod: "webauthn" },
        { accessToken },
      );

      setIsLoading(false);

      if (prefErr) {
        console.error("Failed to set preferred method:", prefErr);
      }

      toast.success("Passkey has been registered", {
        description:
          "Your passkey or security key is now configured for sign-in.",
      });

      onSuccess();
      handleClose(false);
    } catch (err) {
      setIsLoading(false);

      const message =
        err instanceof Error ? err.message : "Passkey registration cancelled.";

      // NotAllowedError = user cancelled
      if (err instanceof Error && err.name === "NotAllowedError") {
        setError("Registration was cancelled.");
        return;
      }

      setError(message);
    }
  }, [userId, applicationId, accessToken, onSuccess]);

  // Non-TOTP MFA types (email, webauthn) — specific enable dialogs
  if (mfaType !== "totp") {
    return (
      <Dialog open={open} onOpenChange={handleClose}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Icon className="size-5" />
              Enable {label}
            </DialogTitle>

            <DialogDescription>
              {mfaType === "email"
                ? "Enable email-based two-factor authentication. A verification code will be sent to your registered email each time you sign in."
                : "Register a passkey or security key for passwordless authentication."}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-2">
            {error && (
              <Alert variant="destructive" className="bg-red-500/10">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <Alert>
              <AlertDescription>
                {mfaType === "email"
                  ? "Once enabled, you'll receive a 6-digit code to your email address each time MFA is required during sign-in."
                  : "Your browser will prompt you to use a fingerprint reader, security key, or device passkey. Make sure your device supports WebAuthn."}
              </AlertDescription>
            </Alert>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => handleClose(false)}
              disabled={isLoading}
            >
              Cancel
            </Button>

            <Button
              onClick={
                mfaType === "email"
                  ? handleEnableEmailMfa
                  : handleEnableWebAuthn
              }
              disabled={isLoading}
            >
              {isLoading && <Loader2 className="mr-2 size-4 animate-spin" />}
              <ShieldCheck className="mr-2 size-4" />
              {mfaType === "email" ? "Enable Email MFA" : "Register Passkey"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Smartphone className="size-5" />
            Enable Authenticator App
          </DialogTitle>

          <DialogDescription>
            {step === "generate"
              ? "Setting up your authenticator app..."
              : "Scan the QR code with your authenticator app, then enter the verification code."}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {error && (
            <Alert variant="destructive" className="bg-red-500/10">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {step === "generate" && isLoading && (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="size-8 animate-spin text-muted-foreground" />
            </div>
          )}

          {step === "verify" && otpUrl && (
            <>
              <div className="flex flex-col items-center gap-3">
                <div className="rounded-lg border bg-white p-2">
                  <canvas ref={canvasRef} />
                </div>

                <div className="w-full space-y-1">
                  <Label className="text-xs text-muted-foreground">
                    Can&apos;t scan? Enter this key manually:
                  </Label>

                  <code className="flex items-center justify-center gap-4 rounded bg-muted px-3 py-2 text-center text-sm font-mono tracking-widest select-all">
                    {secretKey}
                    <button
                      type="button"
                      title="Copy value"
                      className="hover:bg-muted-foreground/10 rounded p-1"
                      onClick={() => secretKey && copy(secretKey)}
                    >
                      <Copy className="size-4" />
                    </button>
                  </code>
                </div>
              </div>

              <div className="space-y-2">
                <Label>Verification Code</Label>

                <div className="flex justify-center">
                  <InputOTP
                    maxLength={6}
                    value={totpCode}
                    onChange={setTotpCode}
                    onComplete={handleVerifyTotp}
                  >
                    <InputOTPGroup>
                      <InputOTPSlot index={0} />
                      <InputOTPSlot index={1} />
                      <InputOTPSlot index={2} />
                    </InputOTPGroup>

                    <InputOTPSeparator />

                    <InputOTPGroup>
                      <InputOTPSlot index={3} />
                      <InputOTPSlot index={4} />
                      <InputOTPSlot index={5} />
                    </InputOTPGroup>
                  </InputOTP>
                </div>
              </div>
            </>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => handleClose(false)}>
            Cancel
          </Button>

          {step === "verify" && (
            <Button
              onClick={handleVerifyTotp}
              disabled={isLoading || totpCode.length !== 6}
            >
              {isLoading && <Loader2 className="mr-2 size-4 animate-spin" />}
              <ShieldCheck className="mr-2 size-4" />
              Verify & Enable
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
