"use client";

import { useState, useCallback } from "react";
import {
  Monitor,
  Smartphone,
  Globe,
  Loader2,
  Trash2,
  LogOut,
} from "lucide-react";
import { toast } from "sonner";
import { formatDistanceToNow } from "date-fns";

import { useSession } from "../../session-context";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";

import {
  useAccountSessionsSWR,
  type AccountSession,
} from "@/services/account/use-account-sessions-swr";
import { accountRevokeSessionApi } from "@/services/account/revoke-session";
import { accountRevokeAllSessionsApi } from "@/services/account/revoke-all-sessions";

type SessionsDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  applicationId: string;
  stepUpToken: string | null;
  onRequestStepUp: () => Promise<string | null>;
};

function getDeviceIcon(userAgent: string) {
  const ua = userAgent.toLowerCase();
  if (
    ua.includes("mobile") ||
    ua.includes("android") ||
    ua.includes("iphone")
  ) {
    return <Smartphone className="size-5" />;
  }
  return <Monitor className="size-5" />;
}

function parseUserAgent(userAgent: string): string {
  // Simple browser/OS extraction
  if (userAgent.includes("Chrome")) return "Chrome";
  if (userAgent.includes("Firefox")) return "Firefox";
  if (userAgent.includes("Safari")) return "Safari";
  if (userAgent.includes("Edge")) return "Edge";
  return userAgent.slice(0, 40);
}

export function SessionsDialog({
  open,
  onOpenChange,
  applicationId,
  stepUpToken,
  onRequestStepUp,
}: SessionsDialogProps) {
  const { accessToken } = useSession();
  const {
    data,
    error: fetchError,
    isLoading,
    mutate,
  } = useAccountSessionsSWR({ accessToken }, stepUpToken);

  const [revokingId, setRevokingId] = useState<string | null>(null);
  const [isRevokingAll, setIsRevokingAll] = useState(false);

  const handleRevokeSession = useCallback(
    async (sessionId: string) => {
      setRevokingId(sessionId);

      const [err] = await accountRevokeSessionApi(
        { sessionId },
        { accessToken },
      );

      setRevokingId(null);

      if (err) {
        if (err.response?.status === 401) {
          toast.error(
            "Your session has expired. Please go back and sign in again.",
          );
          return;
        }

        toast.error(err.response?.data?.message || "Failed to revoke session");
        return;
      }

      toast.success("Session revoked");
      mutate();
    },
    [accessToken, mutate],
  );

  const handleRevokeAll = useCallback(async () => {
    if (!stepUpToken) {
      const token = await onRequestStepUp();
      if (!token) return;
    }

    setIsRevokingAll(true);

    const [, err] = await accountRevokeAllSessionsApi(
      { applicationId, stepUpToken: stepUpToken! },
      { accessToken },
    );

    setIsRevokingAll(false);

    if (err) {
      if (err.response?.status === 401) {
        toast.error(
          "Your session has expired. Please go back and sign in again.",
        );
        return;
      }

      toast.error(
        err.response?.data?.message || "Failed to revoke all sessions",
      );
      return;
    }

    toast.success("All other sessions revoked");
    mutate();
  }, [applicationId, accessToken, stepUpToken, onRequestStepUp, mutate]);

  const sessions = data?.sessions ?? [];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Globe className="size-5" />
            Active Sessions
          </DialogTitle>

          <DialogDescription>
            Manage devices and browsers that are signed into your account.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {fetchError && (
            <Alert variant="destructive" className="bg-red-500/10">
              <AlertDescription>
                {fetchError.response?.status === 401
                  ? "Your session has expired. Please go back and sign in again."
                  : fetchError.response?.data?.message ||
                    "Failed to load sessions."}
              </AlertDescription>
            </Alert>
          )}

          {!stepUpToken && !isLoading && (
            <div className="flex flex-col items-center gap-3 py-6">
              <p className="text-sm text-muted-foreground text-center">
                Viewing active sessions requires identity verification.
              </p>

              <Button onClick={onRequestStepUp}>Verify Identity</Button>
            </div>
          )}

          {isLoading && (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="size-8 animate-spin text-muted-foreground" />
            </div>
          )}

          {stepUpToken && !isLoading && sessions.length > 0 && (
            <ScrollArea className="max-h-80">
              <div className="space-y-3">
                {sessions.map((session: AccountSession, index: number) => (
                  <div key={session.id}>
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex items-start gap-3">
                        <div className="mt-0.5 text-muted-foreground">
                          {getDeviceIcon(session.userAgent)}
                        </div>

                        <div className="space-y-1">
                          <p className="text-sm font-medium">
                            {parseUserAgent(session.userAgent)}
                          </p>

                          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                            <span>{session.ipAddress}</span>

                            {session.location && (
                              <>
                                <span>·</span>
                                <span>{session.location}</span>
                              </>
                            )}
                          </div>

                          <p className="text-xs text-muted-foreground">
                            Last active{" "}
                            {formatDistanceToNow(
                              new Date(session.lastActiveAt),
                              { addSuffix: true },
                            )}
                          </p>
                        </div>
                      </div>

                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8 text-destructive hover:text-destructive"
                        onClick={() => handleRevokeSession(session.id)}
                        disabled={revokingId === session.id}
                      >
                        {revokingId === session.id ? (
                          <Loader2 className="size-4 animate-spin" />
                        ) : (
                          <Trash2 className="size-4" />
                        )}
                      </Button>
                    </div>

                    {index < sessions.length - 1 && (
                      <Separator className="mt-3" />
                    )}
                  </div>
                ))}
              </div>
            </ScrollArea>
          )}

          {stepUpToken && !isLoading && sessions.length === 0 && (
            <p className="py-6 text-center text-sm text-muted-foreground">
              No other active sessions found.
            </p>
          )}
        </div>

        <DialogFooter className="flex-col gap-2 sm:flex-row">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Close
          </Button>

          {stepUpToken && sessions.length > 0 && (
            <Button
              variant="destructive"
              onClick={handleRevokeAll}
              disabled={isRevokingAll}
            >
              {isRevokingAll ? (
                <Loader2 className="mr-2 size-4 animate-spin" />
              ) : (
                <LogOut className="mr-2 size-4" />
              )}
              Revoke All Other Sessions
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
