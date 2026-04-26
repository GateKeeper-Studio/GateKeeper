import { Metadata } from "next";

import { DashboardHeader } from "@/components/dashboard-header";
import { ErrorAlert } from "@/components/error-alert";

import { getTenantByIdService } from "@/services/dashboard/get-tenant-by-id";

import { EditTenantContent } from "./(components)/edit-tenant-content";

type Props = {
  params: Promise<{
    tenantId: string;
  }>;
};

export const metadata: Metadata = {
  title: "Edit Tenant - GateKeeper",
};

export default async function EditTenantPage({ params }: Props) {
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
              name: tenantData?.name || "Tenant",
              path: `/dashboard/tenants/${tenantId}`,
            },
            {
              name: "Edit",
              path: `/dashboard/tenants/${tenantId}/edit`,
            },
          ],
        }}
      />

      <EditTenantContent
        tenant={{
          id: tenantId,
          name: tenantData?.name || "",
          description: tenantData?.description || "",
        }}
      />
    </>
  );
}
