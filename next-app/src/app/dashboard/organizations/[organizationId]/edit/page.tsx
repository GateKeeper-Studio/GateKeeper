import { Metadata } from "next";

import { DashboardHeader } from "@/components/dashboard-header";
import { ErrorAlert } from "@/components/error-alert";

import { getOrganizationByIdService } from "@/services/dashboard/get-organization-by-id";

import { EditOrganizationContent } from "./(components)/edit-organization-content";

type Props = {
  params: Promise<{
    organizationId: string;
  }>;
};

export const metadata: Metadata = {
  title: "Edit Organization - GateKeeper",
};

export default async function EditOrganizationPage({ params }: Props) {
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
              name: organizationData?.name || "Organization",
              path: `/dashboard/organizations/${organizationId}`,
            },
            {
              name: "Edit",
              path: `/dashboard/organizations/${organizationId}/edit`,
            },
          ],
        }}
      />

      <EditOrganizationContent
        organization={{
          id: organizationId,
          name: organizationData?.name || "",
          description: organizationData?.description || "",
        }}
      />
    </>
  );
}
