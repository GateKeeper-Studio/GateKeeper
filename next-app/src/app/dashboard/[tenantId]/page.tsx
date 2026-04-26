import { Metadata } from "next";

import { ErrorAlert } from "@/components/error-alert";
import { DashboardHeader } from "@/components/dashboard-header";

import { getTenantByIdService } from "@/services/dashboard/get-tenant-by-id";
import { TenantTabs } from "./(components)/tenant-tabs";

type Props = {
  params: Promise<{
    tenantId: string;
  }>;
};

export const metadata: Metadata = {
  title: "Tenants - GateKeeper",
};

export default async function TenantPage({ params }: Props) {
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
            {
              name: tenantData?.name || "Tenant Details",
              path: `/dashboard/${tenantId}`,
            },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <h2 className="text-3xl font-bold tracking-tight">
          {tenantData?.name}
        </h2>

        <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
          {tenantData?.description || "No description provided."}
        </span>

        <TenantTabs />
      </main>
    </>
  );
}
