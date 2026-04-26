import { cookies } from "next/headers";
import { Metadata } from "next";

import { DashboardHeader } from "@/components/dashboard-header";
import { UsersTab } from "../application/[applicationId]/(components)/application-details-content/users-tab";

export const metadata: Metadata = {
  title: "Users - GateKeeper",
};

type Props = {
  params: Promise<{
    tenantId: string;
  }>;
};

export default async function UsersPage({ params }: Props) {
  const { tenantId } = await params;
  const tenantName =
    (await cookies()).get("tenant")?.value || "Tenant Detail";

  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: "/dashboard" },
            { name: tenantName, path: `/dashboard/${tenantId}` },
            { name: "Users", path: `/dashboard/${tenantId}/users` },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <UsersTab />
      </main>
    </>
  );
}
