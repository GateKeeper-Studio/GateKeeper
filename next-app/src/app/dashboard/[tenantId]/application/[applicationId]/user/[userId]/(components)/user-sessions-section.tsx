"use client";

import { toast } from "sonner";
import { useState } from "react";
import { useParams } from "next/navigation";
import { Ban, Monitor, ShieldCheck, ShieldOff } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button, buttonVariants } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import {
  useUserSessionsSWR,
  type UserSession,
} from "@/services/dashboard/use-user-sessions-swr";
import { revokeUserSessionApi } from "@/services/dashboard/revoke-user-session";

export function UserSessionsSection() {
  const { tenantId, userId } = useParams() as {
    tenantId: string;
    userId: string;
  };

  const { data, isLoading, mutate } = useUserSessionsSWR(
    { tenantId, userId },
    { accessToken: "" },
  );

  const [revokeTarget, setRevokeTarget] = useState<UserSession | null>(null);
  const [isRevoking, setIsRevoking] = useState(false);

  async function handleRevoke() {
    if (!revokeTarget) return;

    setIsRevoking(true);

    const [err] = await revokeUserSessionApi(
      {
        tenantId,
        userId,
        sessionId: revokeTarget.id,
      },
      { accessToken: "" },
    );

    if (err) {
      console.error(err);
      toast.error(err?.response?.data?.message || "Failed to revoke session.");
      setIsRevoking(false);
      return;
    }

    toast.success("Session revoked successfully.");

    mutate(
      data
        ? {
            ...data,
            data: data.data.filter((s) => s.id !== revokeTarget.id),
          }
        : undefined,
      { revalidate: true },
    );

    setIsRevoking(false);
    setRevokeTarget(null);
  }

  const activeSessions =
    data?.data?.filter((session) => session.isActive) || [];
  const expiredSessions =
    data?.data?.filter((session) => !session.isActive) || [];

  function formatDate(dateString: string) {
    return new Date(dateString).toLocaleString();
  }

  function getTimeRemaining(expiresAt: string) {
    const now = new Date();
    const exp = new Date(expiresAt);
    const diff = exp.getTime() - now.getTime();

    if (diff <= 0) return "Expired";

    const hours = Math.floor(diff / (1000 * 60 * 60));
    const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));

    if (hours > 24) {
      const days = Math.floor(hours / 24);
      return `${days}d ${hours % 24}h remaining`;
    }

    return `${hours}h ${minutes}m remaining`;
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <Monitor className="h-4 w-4" />
        <span className="text-sm font-medium">Active Sessions</span>

        {data && (
          <Badge variant="outline" className="ml-1">
            {activeSessions.length} active
          </Badge>
        )}
      </div>

      <span className="text-muted-foreground text-sm">
        Manage the user&apos;s active sessions. You can revoke any session to
        force the user to sign in again.
      </span>

      {isLoading && (
        <div className="flex flex-col gap-2 mt-2">
          <Skeleton className="w-full h-12" />
          <Skeleton className="w-full h-12" />
        </div>
      )}

      {!isLoading && data?.data?.length === 0 && (
        <div className="mt-2 rounded-lg border border-dashed p-6 text-center text-muted-foreground">
          No sessions found for this user.
        </div>
      )}

      {!isLoading && data && data.data.length > 0 && (
        <div className="mt-2 rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Status</TableHead>
                <TableHead>Created At</TableHead>
                <TableHead>Expires At</TableHead>
                <TableHead>Time Remaining</TableHead>
                <TableHead>Actions</TableHead>
              </TableRow>
            </TableHeader>

            <TableBody>
              {activeSessions.map((session) => (
                <TableRow key={session.id}>
                  <TableCell>
                    <Badge
                      variant="default"
                      className="bg-green-600 hover:bg-green-700"
                    >
                      <ShieldCheck className="h-3 w-3 mr-1" />
                      Active
                    </Badge>
                  </TableCell>

                  <TableCell className="text-sm">
                    {formatDate(session.createdAt)}
                  </TableCell>

                  <TableCell className="text-sm">
                    {formatDate(session.expiresAt)}
                  </TableCell>

                  <TableCell className="text-sm text-green-600 font-medium">
                    {getTimeRemaining(session.expiresAt)}
                  </TableCell>

                  <TableCell className="text-right">
                    <Tooltip delayDuration={0}>
                      <TooltipTrigger asChild>
                        <Button
                          variant="destructive"
                          size="sm"
                          onClick={() => setRevokeTarget(session)}
                        >
                          <Ban className="h-3 w-3 mr-1" />
                          Revoke
                        </Button>
                      </TooltipTrigger>

                      <TooltipContent>
                        Revoke this session to force re-authentication
                      </TooltipContent>
                    </Tooltip>
                  </TableCell>
                </TableRow>
              ))}

              {expiredSessions.map((session) => (
                <TableRow key={session.id} className="opacity-50">
                  <TableCell>
                    <Badge variant="secondary">
                      <ShieldOff className="h-3 w-3 mr-1" />
                      Expired
                    </Badge>
                  </TableCell>

                  <TableCell className="text-sm">
                    {formatDate(session.createdAt)}
                  </TableCell>

                  <TableCell className="text-sm">
                    {formatDate(session.expiresAt)}
                  </TableCell>

                  <TableCell className="text-sm text-muted-foreground">
                    Expired
                  </TableCell>

                  <TableCell className="text-right">
                    <Button variant="outline" size="sm" disabled>
                      <Ban className="h-3 w-3 mr-1" />
                      Expired
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      <Dialog
        open={!!revokeTarget}
        onOpenChange={(open) => !open && setRevokeTarget(null)}
      >
        <DialogContent className="sm:max-w-112.5">
          <DialogHeader>
            <DialogTitle>Revoke Session</DialogTitle>

            <DialogDescription>
              Are you sure you want to revoke this session? The user will be
              signed out and will need to authenticate again.
            </DialogDescription>
          </DialogHeader>

          <DialogFooter>
            <DialogClose className={buttonVariants({ variant: "outline" })}>
              Cancel
            </DialogClose>

            <Button
              variant="destructive"
              onClick={handleRevoke}
              disabled={isRevoking}
            >
              {isRevoking ? "Revoking..." : "Revoke Session"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
