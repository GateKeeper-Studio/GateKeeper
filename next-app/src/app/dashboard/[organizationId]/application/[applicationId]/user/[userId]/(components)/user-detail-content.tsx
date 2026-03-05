"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { Copy, Monitor, Pencil, ShieldCheck, ShieldOff } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { buttonVariants } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { cn, copy } from "@/lib/utils";

import { DeleteUserDialog } from "./delete-user-dialog";
import { UserSessionsSection } from "./user-sessions-section";

import { UserByIdResponse } from "@/services/dashboard/get-tenant-user-by-id";

type Props = {
  user: UserByIdResponse | null;
};

export function UserDetailContent({ user }: Props) {
  const { organizationId, userId } = useParams() as {
    organizationId: string;
    userId: string;
  };

  return (
    <>
      <div className="flex items-center justify-between gap-4">
        <div className="flex gap-4 items-center">
          <h2 className="text-3xl font-bold tracking-tight">
            {user?.displayName}
          </h2>

          {user?.isActive ? (
            <Tooltip>
              <TooltipTrigger className="flex gap-2">
                <Badge variant="default" className="text-sm h-6 px-2">
                  <ShieldCheck size="1.5rem" />
                  Active
                </Badge>
              </TooltipTrigger>

              <TooltipContent>User account is active</TooltipContent>
            </Tooltip>
          ) : (
            <Tooltip>
              <TooltipTrigger className="flex gap-2">
                <Badge variant="secondary" className="text-sm h-6 px-2">
                  <ShieldOff size="1.5rem" />
                  Disabled
                </Badge>
              </TooltipTrigger>

              <TooltipContent>User account is disabled</TooltipContent>
            </Tooltip>
          )}
        </div>

        <div className="flex gap-1">
          <DeleteUserDialog />

          <Tooltip delayDuration={0}>
            <TooltipTrigger
              className={cn(buttonVariants({ variant: "outline" }))}
              asChild
            >
              <Link
                href={`/dashboard/${organizationId}/users/${userId}/edit-user`}
              >
                <Pencil />
              </Link>
            </TooltipTrigger>

            <TooltipContent>Edit User</TooltipContent>
          </Tooltip>
        </div>
      </div>

      <div className="mt-6 max-w-[700px] flex flex-col gap-4">
        <fieldset className="flex gap-4">
          <div className="flex flex-col gap-1 w-full">
            <span className="text-sm font-medium">First Name</span>
            <div className="flex gap-2 items-center">
              <span className="text-sm text-muted-foreground">
                {user?.firstName || "-"}
              </span>

              <Tooltip delayDuration={0}>
                <TooltipTrigger
                  className={cn(
                    buttonVariants({ variant: "ghost", size: "icon" }),
                    "h-7 w-7",
                  )}
                  onClick={() => copy(user?.firstName || "")}
                >
                  <Copy className="h-3.5 w-3.5" />
                </TooltipTrigger>

                <TooltipContent>Copy first name</TooltipContent>
              </Tooltip>
            </div>
          </div>

          <div className="flex flex-col gap-1 w-full">
            <span className="text-sm font-medium">Last Name</span>
            <div className="flex gap-2 items-center">
              <span className="text-sm text-muted-foreground">
                {user?.lastName || "-"}
              </span>

              <Tooltip delayDuration={0}>
                <TooltipTrigger
                  className={cn(
                    buttonVariants({ variant: "ghost", size: "icon" }),
                    "h-7 w-7",
                  )}
                  onClick={() => copy(user?.lastName || "")}
                >
                  <Copy className="h-3.5 w-3.5" />
                </TooltipTrigger>

                <TooltipContent>Copy last name</TooltipContent>
              </Tooltip>
            </div>
          </div>
        </fieldset>

        <Separator />

        <div className="flex flex-col gap-1">
          <span className="text-sm font-medium">E-mail</span>
          <div className="flex gap-2 items-center">
            <span className="text-sm text-muted-foreground">
              {user?.email || "-"}
            </span>

            <Tooltip delayDuration={0}>
              <TooltipTrigger
                className={cn(
                  buttonVariants({ variant: "ghost", size: "icon" }),
                  "h-7 w-7",
                )}
                onClick={() => copy(user?.email || "")}
              >
                <Copy className="h-3.5 w-3.5" />
              </TooltipTrigger>

              <TooltipContent>Copy e-mail</TooltipContent>
            </Tooltip>
          </div>
        </div>

        <div className="w-full p-3 rounded-lg bg-gray-50 items-center dark:bg-gray-900 shadow flex gap-2">
          <span className="text-primary-background text-sm">
            Is e-mail already confirmed?
          </span>

          {user?.isEmailVerified ? (
            <span className="text-green-500 font-semibold">Yes</span>
          ) : (
            <span className="text-red-500 font-semibold">No</span>
          )}
        </div>

        <Separator />

        <div className="flex flex-col gap-1">
          <span className="text-sm font-medium">Preferred MFA Method</span>
          <span className="text-sm text-muted-foreground">
            {user?.preferred2FAMethod
              ? {
                  email: "E-mail",
                  totp: "Authenticator App",
                  sms: "SMS",
                  webauthn: "Passkey (WebAuthn)",
                }[user.preferred2FAMethod] || user.preferred2FAMethod
              : "None"}
          </span>
        </div>

        <ul className="flex flex-col gap-1 mt-1">
          <li className="flex items-center gap-4">
            <span className="text-muted-background font-semibold text-sm">
              E-mail
            </span>

            {user?.isMfaEmailConfigured ? (
              <span className="text-sm bg-green-300/30 px-3 rounded-full border border-green-300 text-green-900 dark:text-green-300">
                Configured
              </span>
            ) : (
              <span className="text-sm bg-red-300/30 px-3 rounded-full border border-red-300 text-red-900 dark:text-red-300">
                Not Configured
              </span>
            )}
          </li>

          <li className="flex items-center gap-4">
            <span className="text-muted-background font-semibold text-sm">
              Authenticator App
            </span>

            {user?.isMfaAuthAppConfigured ? (
              <span className="text-sm bg-green-300/30 px-3 rounded-full border border-green-300 text-green-900 dark:text-green-300">
                Configured
              </span>
            ) : (
              <span className="text-sm bg-red-300/30 px-3 rounded-full border border-red-300 text-red-900 dark:text-red-300">
                Not Configured
              </span>
            )}
          </li>

          <li className="flex items-center gap-4">
            <span className="text-muted-background font-semibold text-sm">
              Passkey (WebAuthn)
            </span>

            {user?.isMfaWebauthnConfigured ? (
              <span className="text-sm bg-green-300/30 px-3 rounded-full border border-green-300 text-green-900 dark:text-green-300">
                Configured
              </span>
            ) : (
              <span className="text-sm bg-red-300/30 px-3 rounded-full border border-red-300 text-red-900 dark:text-red-300">
                Not Configured
              </span>
            )}
          </li>
        </ul>

        <Separator />

        <div className="flex flex-col gap-1">
          <span className="text-sm font-medium">Application Roles</span>
          <span className="text-muted-foreground my-1 text-sm">
            Roles assigned to this user.
          </span>

          {user?.badges && user.badges.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {user.badges.map((role) => (
                <Badge key={role.id} variant="outline">
                  {role.name}
                </Badge>
              ))}
            </div>
          ) : (
            <span className="text-sm text-muted-foreground italic">
              No roles assigned.
            </span>
          )}
        </div>

        <Separator />

        <UserSessionsSection />
      </div>
    </>
  );
}
