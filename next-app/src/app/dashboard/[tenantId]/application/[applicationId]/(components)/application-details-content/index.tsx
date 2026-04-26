"use client";

import Link from "next/link";
import {
  BadgeCent,
  BadgeInfo,
  ChevronLeft,
  Pencil,
  ShieldCheck,
  ShieldOff,
} from "lucide-react";

import { buttonVariants } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Badge } from "@/components/ui/badge";
import { DashboardHeader } from "@/components/dashboard-header";

import { cn } from "@/lib/utils";

import { ApplicationTabs } from "./application-tabs";
import { DeleteApplicationDialog } from "./roles/delete-application-dialog";

import { useTenantsContext } from "@/app/dashboard/(contexts)/tenants-context-provider";
import { useApplicationContext } from "../../(contexts)/application-context-provider";
import { Skeleton } from "@/components/ui/skeleton";

type Props = {
  applicationId: string;
  tenantId: string;
};

export function ApplicationDetailsContent({
  applicationId,
  tenantId,
}: Props) {
  const { selectedTenant } = useTenantsContext();
  const { application, isLoading } = useApplicationContext();

  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: `/dashboard` },

            {
              name: selectedTenant?.name || "-",
              path: `/dashboard/${tenantId}`,
            },
            {
              name: "Applications",
              path: `/dashboard/${tenantId}?tab=applications`,
            },
            {
              name: application?.name || "-",
              path: `/dashboard/${tenantId}/application/${applicationId}`,
            },
          ],
        }}
      />

      <main className="flex flex-col p-4 flex-1">
        <Link
          href={`/dashboard/${tenantId}?tab=applications`}
          className="text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:text-gray-800 hover:underline"
        >
          <ChevronLeft size={24} />
          Go back to applications list
        </Link>

        <div className="flex items-center justify-between gap-4">
          <div className="flex gap-4 items-center">
            {isLoading ? (
              <>
                <Skeleton className="h-9 w-60" />
                <Skeleton className="h-7 w-16" />
              </>
            ) : (
              <>
                <h2 className="text-3xl font-bold tracking-tight">
                  {application?.name}
                </h2>

                {application?.isActive ? (
                  <Tooltip>
                    <TooltipTrigger className="flex gap-2">
                      <Badge variant="default" className="text-sm h-6 px-2">
                        <ShieldCheck size="1.5rem" />
                        Active
                      </Badge>
                    </TooltipTrigger>

                    <TooltipContent>
                      Application Project, everything is functioning normally
                    </TooltipContent>
                  </Tooltip>
                ) : (
                  <Tooltip>
                    <TooltipTrigger className="flex gap-2">
                      <ShieldOff size="1.5rem" />
                      <Badge variant="secondary" className="text-sm h-6 px-2">
                        Inactive
                      </Badge>
                    </TooltipTrigger>

                    <TooltipContent>
                      Inactive Application, not currently in use. Can be viewed
                      but not modified
                    </TooltipContent>
                  </Tooltip>
                )}
              </>
            )}
          </div>

          <div className="flex gap-1">
            <Tooltip>
              <TooltipTrigger asChild>
                <DeleteApplicationDialog application={application} />
              </TooltipTrigger>

              <TooltipContent>
                <p>Delete Application</p>
              </TooltipContent>
            </Tooltip>

            <Tooltip delayDuration={0}>
              <TooltipTrigger
                className={cn(buttonVariants({ variant: "outline" }))}
                asChild
              >
                <Link
                  href={`/dashboard/${tenantId}/application/${applicationId}/edit-application`}
                >
                  <Pencil />
                </Link>
              </TooltipTrigger>

              <TooltipContent>
                <p>Update Application</p>
              </TooltipContent>
            </Tooltip>
          </div>
        </div>

        <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
          {application?.description}
        </span>

        <div className="mt-4 flex flex-wrap gap-2">
          {application?.badges.map((badge, i) => (
            <Badge variant="outline" key={i}>
              <BadgeInfo />
              {badge}
            </Badge>
          ))}
        </div>

        <ApplicationTabs
          application={application}
          tenantId={tenantId}
        />
      </main>
    </>
  );
}
