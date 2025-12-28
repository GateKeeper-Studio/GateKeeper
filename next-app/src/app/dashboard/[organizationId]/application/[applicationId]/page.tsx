import Link from "next/link";
import { ChevronLeft, Pencil } from "lucide-react";

import { Breadcrumbs } from "@/components/bread-crumbs";
import { buttonVariants } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Badge } from "@/components/ui/badge";
import { ErrorAlert } from "@/components/error-alert";

import { cn } from "@/lib/utils";
import { getApplicationByIdService } from "@/services/dashboard/get-application-by-id";

import { ApplicationTabs } from "./(components)/application-tabs";
import { DeleteApplicationDialog } from "./(components)/delete-application-dialog";
import { cookies } from "next/headers";

type Props = {
  params: Promise<{
    applicationId: string;
    organizationId: string;
  }>;
};

export async function generateMetadata({ params }: Props) {
  const { applicationId, organizationId } = await params;

  const [application, err] = await getApplicationByIdService(
    { applicationId, organizationId },
    { accessToken: "" }
  );

  if (err) {
    return {
      title: "Application - GateKeeper",
    };
  }

  return {
    title: `${application?.name} - Application - GateKeeper`,
  };
}

export default async function ApplicationDetailPage({ params }: Props) {
  const { applicationId, organizationId } = await params;

  const [application, err] = await getApplicationByIdService(
    { applicationId, organizationId },
    { accessToken: "" }
  );

  const organizationName = (await cookies()).get("organization")?.value || "Organization Detail";

  if (err) {
    return (
      <div className="flex m-4 w-full">
        <ErrorAlert
          message={
            err.response?.data.message ||
            "Failed on trying to fetch application"
          }
          title={err.response?.data.title || "An Error Occurred"}
        />
      </div>
    );
  }

  return (
    <>
      <Breadcrumbs
        items={[
          { name: "Dashboard", path: `/dashboard` },

          { name: organizationName, path: `/dashboard/${organizationId}` },
          {
            name: "Applications",
            path: `/dashboard/${organizationId}/application`,
          },
          {
            name: application?.name || "-",
            path: `/dashboard/${organizationId}/application/${applicationId}`,
          },
        ]}
      />

      <main className="flex flex-col p-4">
        <Link
          href={`/dashboard/${organizationId}/application`}
          className="text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:text-gray-800 hover:underline"
        >
          <ChevronLeft size={24} />
          Go back to applications list
        </Link>

        <div className="flex items-center justify-between gap-4">
          <h2 className="text-3xl font-bold tracking-tight">
            {application?.name}
          </h2>

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
                  href={`/dashboard/${organizationId}/application/${applicationId}/edit-application`}
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
              {badge}
            </Badge>
          ))}
        </div>

        <ApplicationTabs
          application={application}
          organizationId={organizationId}
        />
      </main>
    </>
  );
}
