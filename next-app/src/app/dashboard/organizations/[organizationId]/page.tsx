import { Metadata } from "next";

import { DashboardHeader } from "@/components/dashboard-header";

import { ViewOrganizationContent } from "./(components)/view-organization-content";

import { getOrganizationByIdService } from "@/services/dashboard/get-organization-by-id";
import { ErrorAlert } from "@/components/error-alert";

type Props = {
  params: Promise<{
    organizationId: string;
  }>;
};

export const metadata: Metadata = {
  title: "View Organization - GateKeeper",
};

export default async function ViewOrganizationPage({ params }: Props) {
  const { organizationId } = await params;

  const [organizationData, err] = await getOrganizationByIdService(
    { organizationId },
    { accessToken: "" },
  );

  if (err) {
    return (
      <div className="flex m-4 w-full">
        <ErrorAlert
          message={
            err.response?.data.message ||
            "Failed on trying to fetch organization"
          }
          title={err.response?.data.title || "An Error Occurred"}
        />
      </div>
    );
  }

  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: "/dashboard" },
            { name: "Organizations", path: "/dashboard/organizations" },
            {
              name: organizationData?.name || "Organization Details",
              path: `/dashboard/organizations/${organizationId}`,
            },
          ],
        }}
      />

      <ViewOrganizationContent
        organizationId={organizationId}
        initialName={organizationData?.name || ""}
        initialDescription={organizationData?.description || ""}
      />
    </>
  );
}
