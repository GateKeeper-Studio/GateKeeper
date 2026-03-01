import { Metadata } from "next";

import { DashboardHeader } from "@/components/dashboard-header";

import { CreateOrganizationContent } from "./(components)/create-organization-content";

export const metadata: Metadata = {
  title: "Create Organization - GateKeeper",
};

export default function CreateOrganizationPage() {
  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: "/dashboard" },
            { name: "Organizations", path: "/dashboard/organizations" },
            {
              name: "Create Organization",
              path: "/dashboard/organizations/create",
            },
          ],
        }}
      />

      <CreateOrganizationContent />
    </>
  );
}
