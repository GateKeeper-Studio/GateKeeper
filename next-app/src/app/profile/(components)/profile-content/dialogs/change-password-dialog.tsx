"use client";

import { useState, useCallback } from "react";
import { Eye, EyeOff, Key, Loader2, Check } from "lucide-react";
import { toast } from "sonner";

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
import { Alert, AlertDescription } from "@/components/ui/alert";

import { accountChangePasswordApi } from "@/services/account/change-password";
import { useSession } from "../../session-context";

type ChangePasswordDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  applicationId: string;
  stepUpToken: string;
};

export function ChangePasswordDialog({
  open,
  onOpenChange,
  applicationId,
  stepUpToken,
}: ChangePasswordDialogProps) {
  const { accessToken } = useSession();
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showCurrentPassword, setShowCurrentPassword] = useState(false);
  const [showNewPassword, setShowNewPassword] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const passwordsMatch = newPassword === confirmPassword;
  const isStrongPassword =
    newPassword.length >= 8 &&
    /[A-Z]/.test(newPassword) &&
    /[a-z]/.test(newPassword) &&
    /[0-9]/.test(newPassword);

  const handleSubmit = useCallback(async () => {
    if (!currentPassword) {
      setError("Current password is required");
      return;
    }

    if (!newPassword) {
      setError("New password is required");
      return;
    }

    if (!isStrongPassword) {
      setError(
        "Password must be at least 8 characters with uppercase, lowercase, and a number",
      );
      return;
    }

    if (!passwordsMatch) {
      setError("Passwords do not match");
      return;
    }

    setIsLoading(true);
    setError(null);

    const [, err] = await accountChangePasswordApi(
      { applicationId, currentPassword, newPassword, stepUpToken },
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
          "Failed to change password. Please try again.",
      );
      return;
    }

    toast.success("Password changed successfully", {
      description: "All other sessions have been revoked for security.",
    });

    handleClose(false);
  }, [
    currentPassword,
    newPassword,
    confirmPassword,
    isStrongPassword,
    passwordsMatch,
    applicationId,
    accessToken,
    stepUpToken,
  ]);

  const handleClose = (open: boolean) => {
    if (!open) {
      setCurrentPassword("");
      setNewPassword("");
      setConfirmPassword("");
      setShowCurrentPassword(false);
      setShowNewPassword(false);
      setError(null);
    }
    onOpenChange(open);
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Key className="size-5" />
            Change Password
          </DialogTitle>

          <DialogDescription>
            Enter your current password and choose a new one.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {error && (
            <Alert variant="destructive" className="bg-red-500/10">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <div className="space-y-2">
            <Label htmlFor="current-password">Current Password</Label>

            <div className="relative">
              <Input
                id="current-password"
                type={showCurrentPassword ? "text" : "password"}
                value={currentPassword}
                onChange={(e) => setCurrentPassword(e.target.value)}
                placeholder="Enter current password"
                autoFocus
              />

              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="absolute right-1 top-1/2 -translate-y-1/2 h-7 w-7"
                onClick={() => setShowCurrentPassword(!showCurrentPassword)}
              >
                {showCurrentPassword ? (
                  <EyeOff className="size-4" />
                ) : (
                  <Eye className="size-4" />
                )}
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="new-password">New Password</Label>

            <div className="relative">
              <Input
                id="new-password"
                type={showNewPassword ? "text" : "password"}
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                placeholder="Enter new password"
              />

              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="absolute right-1 top-1/2 -translate-y-1/2 h-7 w-7"
                onClick={() => setShowNewPassword(!showNewPassword)}
              >
                {showNewPassword ? (
                  <EyeOff className="size-4" />
                ) : (
                  <Eye className="size-4" />
                )}
              </Button>
            </div>

            {newPassword && (
              <div className="space-y-1 text-xs">
                <div
                  className={
                    newPassword.length >= 8
                      ? "text-green-600"
                      : "text-muted-foreground"
                  }
                >
                  {newPassword.length >= 8 ? (
                    <Check className="mr-1 inline size-3" />
                  ) : (
                    "○ "
                  )}
                  At least 8 characters
                </div>

                <div
                  className={
                    /[A-Z]/.test(newPassword) && /[a-z]/.test(newPassword)
                      ? "text-green-600"
                      : "text-muted-foreground"
                  }
                >
                  {/[A-Z]/.test(newPassword) && /[a-z]/.test(newPassword) ? (
                    <Check className="mr-1 inline size-3" />
                  ) : (
                    "○ "
                  )}
                  Uppercase and lowercase letters
                </div>

                <div
                  className={
                    /[0-9]/.test(newPassword)
                      ? "text-green-600"
                      : "text-muted-foreground"
                  }
                >
                  {/[0-9]/.test(newPassword) ? (
                    <Check className="mr-1 inline size-3" />
                  ) : (
                    "○ "
                  )}
                  At least one number
                </div>
              </div>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="confirm-password">Confirm New Password</Label>

            <Input
              id="confirm-password"
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              placeholder="Confirm new password"
              onKeyDown={(e) => {
                if (e.key === "Enter") handleSubmit();
              }}
            />

            {confirmPassword && !passwordsMatch && (
              <p className="text-xs text-destructive">Passwords do not match</p>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => handleClose(false)}>
            Cancel
          </Button>

          <Button
            onClick={handleSubmit}
            disabled={isLoading || !passwordsMatch || !isStrongPassword}
          >
            {isLoading && <Loader2 className="mr-2 size-4 animate-spin" />}
            Change Password
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
