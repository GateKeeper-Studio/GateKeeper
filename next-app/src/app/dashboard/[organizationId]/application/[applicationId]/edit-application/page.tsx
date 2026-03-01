import Link from "next/link";
import { ChevronLeft } from "lucide-react";

import { EditApplicationForm } from "./(components)/edit-application-form";
import { getApplicationByIdService } from "@/services/dashboard/get-application-by-id";
import { DashboardHeader } from "@/components/dashboard-header";

type Props = {
  params: Promise<{
    organizationId: string;
    applicationId: string;
  }>;
};

export async function generateMetadata({ params }: Props) {
  const { applicationId, organizationId } = await params;

  const [application, err] = await getApplicationByIdService(
    { applicationId, organizationId },
    { accessToken: "" },
  );

  if (err) {
    return {
      title: "Application - GateKeeper",
    };
  }

  return {
    title: `${application?.name} - GateKeeper`,
  };
}

export default async function EditApplicationPage({ params }: Props) {
  const { applicationId, organizationId } = await params;

  const [application, err] = await getApplicationByIdService(
    { applicationId, organizationId },
    { accessToken: "" },
  );

  if (err) {
    return <div>Failed to fetch application</div>;
  }

  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: `/dashboard/${organizationId}` },
            {
              name: "Applications",
              path: `/dashboard/${organizationId}/application`,
            },
            {
              name: application?.name || "-",
              path: `/dashboard/${organizationId}/application/${applicationId}`,
            },
            {
              name: "Edit Application",
              path: `/dashboard/${organizationId}/application/${applicationId}/edit-application`,
            },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <Link
          href={`/dashboard/${organizationId}/application/${applicationId}`}
          className="text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:dark:text-gray-500 hover:text-gray-800 hover:underline"
        >
          <ChevronLeft size={24} />
          Go back to application details
        </Link>

        <h2 className="text-3xl font-bold tracking-tight">Edit Application</h2>

        <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
          Here you can edit the application. Fill in the form below and click
          the &quot;Apply Changes&quot; button to edit the application.
        </span>

        <EditApplicationForm application={application} />
      </main>
    </>
  );
}
