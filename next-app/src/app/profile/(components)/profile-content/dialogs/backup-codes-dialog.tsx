"use client";

import { useState, useCallback } from "react";
import { Copy, Download, Loader2, ShieldAlert } from "lucide-react";
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
import { Alert, AlertDescription } from "@/components/ui/alert";

import { accountGenerateBackupCodesApi } from "@/services/account/generate-backup-codes";
import { useSession } from "../../session-context";

type BackupCodesDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  applicationId: string;
  stepUpToken: string;
};

export function BackupCodesDialog({
  open,
  onOpenChange,
  applicationId,
  stepUpToken,
}: BackupCodesDialogProps) {
  const { accessToken } = useSession();
  const [codes, setCodes] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const handleGenerate = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    const [data, err] = await accountGenerateBackupCodesApi(
      { applicationId, stepUpToken },
      { accessToken },
    );

    setIsLoading(false);

    if (err) {
      if (err.response?.status === 401) {
        setError("Your session has expired. Please go back and sign in again.");
        return;
      }

      setError(
        err.response?.data?.message || "Failed to generate backup codes.",
      );
      return;
    }

    if (data) {
      setCodes(data.codes);
    }
  }, [applicationId, accessToken, stepUpToken]);

  const handleCopy = () => {
    if (codes) {
      navigator.clipboard.writeText(codes.join("\n"));
      toast.success("Backup codes copied to clipboard");
    }
  };

  const handleDownload = () => {
    if (codes) {
      const content = [
        "GateKeeper - Backup Recovery Codes",
        "=".repeat(40),
        "",
        "Store these codes in a safe place.",
        "Each code can only be used once.",
        "",
        ...codes.map((code, i) => `${i + 1}. ${code}`),
        "",
        `Generated: ${new Date().toISOString()}`,
      ].join("\n");

      const blob = new Blob([content], { type: "text/plain" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "gatekeeper-backup-codes.txt";
      a.click();
      URL.revokeObjectURL(url);
    }
  };

  const handleClose = (open: boolean) => {
    if (!open) {
      setCodes(null);
      setError(null);
    }
    onOpenChange(open);
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ShieldAlert className="size-5" />
            Backup Recovery Codes
          </DialogTitle>

          <DialogDescription>
            {codes
              ? "Save these codes in a safe place. Each code can only be used once to sign in if you lose access to your authenticator."
              : "Generate new backup codes to use as a fallback for two-factor authentication."}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {error && (
            <Alert variant="destructive" className="bg-red-500/10">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {!codes && !isLoading && (
            <Alert className="bg-amber-500/10 border-amber-500/30">
              <AlertDescription className="text-amber-700 dark:text-amber-400">
                Generating new codes will invalidate all previous backup codes.
              </AlertDescription>
            </Alert>
          )}

          {isLoading && (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="size-8 animate-spin text-muted-foreground" />
            </div>
          )}

          {codes && (
            <>
              <div className="grid grid-cols-2 gap-2 rounded-lg border bg-muted/50 p-4">
                {codes.map((code, index) => (
                  <code
                    key={index}
                    className="rounded bg-background px-3 py-1.5 text-center text-sm font-mono tracking-widest"
                  >
                    {code}
                  </code>
                ))}
              </div>

              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleCopy}
                  className="flex-1"
                >
                  <Copy className="mr-2 size-4" />
                  Copy All
                </Button>

                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleDownload}
                  className="flex-1"
                >
                  <Download className="mr-2 size-4" />
                  Download
                </Button>
              </div>
            </>
          )}
        </div>

        <DialogFooter>
          {!codes ? (
            <>
              <Button variant="outline" onClick={() => handleClose(false)}>
                Cancel
              </Button>

              <Button onClick={handleGenerate} disabled={isLoading}>
                {isLoading && <Loader2 className="mr-2 size-4 animate-spin" />}
                Generate Codes
              </Button>
            </>
          ) : (
            <Button onClick={() => handleClose(false)}>
              I&apos;ve Saved My Codes
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
