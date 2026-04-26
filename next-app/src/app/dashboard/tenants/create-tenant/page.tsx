import { Metadata } from "next";

import { DashboardHeader } from "@/components/dashboard-header";

import { CreateTenantContent } from "./(components)/create-tenant-content";

export const metadata: Metadata = {
  title: "Create Tenant - GateKeeper",
};

export default function CreateTenantPage() {
  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: "/dashboard" },
            { name: "Tenants", path: "/dashboard/tenants" },
            {
              name: "Create Tenant",
              path: "/dashboard/tenants/create",
            },
          ],
        }}
      />

      <CreateTenantContent />
    </>
  );
}
