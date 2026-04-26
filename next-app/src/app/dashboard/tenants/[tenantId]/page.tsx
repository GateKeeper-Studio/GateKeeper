import { Metadata } from "next";

import { DashboardHeader } from "@/components/dashboard-header";

import { ViewTenantContent } from "./(components)/view-tenant-content";

import { getTenantByIdService } from "@/services/dashboard/get-tenant-by-id";
import { ErrorAlert } from "@/components/error-alert";

type Props = {
  params: Promise<{
    tenantId: string;
  }>;
};

export const metadata: Metadata = {
  title: "View Tenant - GateKeeper",
};

export default async function ViewTenantPage({ params }: Props) {
  const { tenantId } = await params;

  const [tenantData, err] = await getTenantByIdService(
    { tenantId },
    { accessToken: "" },
  );

  if (err) {
    return (
      <div className="flex m-4 w-full">
        <ErrorAlert
          message={
            err.response?.data.message ||
            "Failed on trying to fetch tenant"
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
            { name: "Tenants", path: "/dashboard/tenants" },
            {
              name: tenantData?.name || "Tenant Details",
              path: `/dashboard/tenants/${tenantId}`,
            },
          ],
        }}
      />

      <ViewTenantContent
        tenantId={tenantId}
        initialName={tenantData?.name || ""}
        initialDescription={tenantData?.description || ""}
      />
    </>
  );
}
