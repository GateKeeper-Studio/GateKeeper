"use client";

import { useState, useCallback } from "react";
import { Mail, Loader2 } from "lucide-react";
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

import { accountRequestEmailChangeApi } from "@/services/account/request-email-change";
import { useSession } from "../../session-context";

type ChangeEmailDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  applicationId: string;
  stepUpToken: string;
  currentEmail: string;
};

export function ChangeEmailDialog({
  open,
  onOpenChange,
  applicationId,
  stepUpToken,
  currentEmail,
}: ChangeEmailDialogProps) {
  const { accessToken } = useSession();
  const [newEmail, setNewEmail] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const isValidEmail = /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(newEmail);
  const isSameEmail = newEmail.toLowerCase() === currentEmail.toLowerCase();

  const handleSubmit = useCallback(async () => {
    if (!newEmail) {
      setError("Email address is required");
      return;
    }

    if (!isValidEmail) {
      setError("Please enter a valid email address");
      return;
    }

    if (isSameEmail) {
      setError("New email must be different from current email");
      return;
    }

    setIsLoading(true);
    setError(null);

    const [data, err] = await accountRequestEmailChangeApi(
      { applicationId, newEmail, stepUpToken },
      { accessToken },
    );

    setIsLoading(false);

    if (err) {
      if (err.response?.status === 401) {
        setError("Your session has expired. Please go back and sign in again.");
        return;
      }

      setError(
        err.response?.data?.message || "Failed to request email change.",
      );
      return;
    }

    if (data) {
      toast.success("Confirmation email sent", {
        description: `A verification link has been sent to ${newEmail}. Please check your inbox.`,
      });

      handleClose(false);
    }
  }, [
    newEmail,
    isValidEmail,
    isSameEmail,
    applicationId,
    accessToken,
    stepUpToken,
  ]);

  const handleClose = (open: boolean) => {
    if (!open) {
      setNewEmail("");
      setError(null);
    }
    onOpenChange(open);
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Mail className="size-5" />
            Change Email Address
          </DialogTitle>

          <DialogDescription>
            A confirmation link will be sent to your new email address.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {error && (
            <Alert variant="destructive" className="bg-red-500/10">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <div className="space-y-2">
            <Label className="text-muted-foreground">Current Email</Label>

            <p className="text-sm">{currentEmail}</p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="new-email">New Email Address</Label>

            <Input
              id="new-email"
              type="email"
              value={newEmail}
              onChange={(e) => setNewEmail(e.target.value)}
              placeholder="Enter new email address"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter") handleSubmit();
              }}
            />

            {isSameEmail && newEmail && (
              <p className="text-xs text-destructive">
                Must be different from current email
              </p>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => handleClose(false)}>
            Cancel
          </Button>

          <Button
            onClick={handleSubmit}
            disabled={isLoading || !isValidEmail || isSameEmail}
          >
            {isLoading && <Loader2 className="mr-2 size-4 animate-spin" />}
            Send Confirmation
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
