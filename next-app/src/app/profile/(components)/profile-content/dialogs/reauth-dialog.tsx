"use client";

import { useState, useCallback } from "react";
import { Eye, EyeOff, ShieldCheck, Loader2 } from "lucide-react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  InputOTP,
  InputOTPGroup,
  InputOTPSlot,
  InputOTPSeparator,
} from "@/components/ui/input-otp";
import { Alert, AlertDescription } from "@/components/ui/alert";

import { reauthenticateApi } from "@/services/account/reauthenticate";
import { useSession } from "../../session-context";

type ReauthDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess: (stepUpToken: string) => void;
  applicationId: string;
  hasMfa?: boolean;
};

export function ReauthDialog({
  open,
  onOpenChange,
  onSuccess,
  applicationId,
  hasMfa = false,
}: ReauthDialogProps) {
  const { accessToken } = useSession();
  const [password, setPassword] = useState("");
  const [totpCode, setTotpCode] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = useCallback(async () => {
    if (!password) {
      setError("Password is required");
      return;
    }

    if (hasMfa && totpCode.length !== 6) {
      setError("Enter your 6-digit authenticator code");
      return;
    }

    setIsLoading(true);
    setError(null);

    const [data, err] = await reauthenticateApi(
      {
        applicationId,
        password,
        totpCode: hasMfa ? totpCode : undefined,
      },
      { accessToken },
    );

    setIsLoading(false);

    if (err) {
      if (err.response?.status === 401) {
        setError("Your session has expired. Please go back and sign in again.");
        return;
      }

      setError(
        err.response?.data?.message ||
          "Authentication failed. Please try again.",
      );
      return;
    }

    if (data) {
      setPassword("");
      setTotpCode("");
      setError(null);
      onSuccess(data.stepUpToken);
      onOpenChange(false);
    }
  }, [
    password,
    totpCode,
    hasMfa,
    applicationId,
    accessToken,
    onSuccess,
    onOpenChange,
  ]);

  const handleClose = (open: boolean) => {
    if (!open) {
      setPassword("");
      setTotpCode("");
      setError(null);
    }
    onOpenChange(open);
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ShieldCheck className="size-5 text-amber-500" />
            Confirm Your Identity
          </DialogTitle>

          <DialogDescription>
            This action requires reauthentication. Enter your current password
            to continue.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {error && (
            <Alert variant="destructive" className="bg-red-500/10">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <div className="space-y-2">
            <Label htmlFor="reauth-password">Current Password</Label>

            <div className="relative">
              <Input
                id="reauth-password"
                type={showPassword ? "text" : "password"}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Enter your password"
                autoFocus
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !hasMfa) handleSubmit();
                }}
              />

              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="absolute right-1 top-1/2 -translate-y-1/2 h-7 w-7"
                onClick={() => setShowPassword(!showPassword)}
              >
                {showPassword ? (
                  <EyeOff className="size-4" />
                ) : (
                  <Eye className="size-4" />
                )}
              </Button>
            </div>
          </div>

          {hasMfa && (
            <div className="space-y-2">
              <Label>Authenticator Code</Label>

              <InputOTP
                maxLength={6}
                value={totpCode}
                onChange={setTotpCode}
                onComplete={handleSubmit}
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
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => handleClose(false)}>
            Cancel
          </Button>

          <Button onClick={handleSubmit} disabled={isLoading}>
            {isLoading && <Loader2 className="mr-2 size-4 animate-spin" />}
            Confirm
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
