import { cookies } from "next/headers";
import { Metadata } from "next";

import { DashboardHeader } from "@/components/dashboard-header";
import { UsersTab } from "../application/[applicationId]/(components)/application-details-content/users-tab";

export const metadata: Metadata = {
  title: "Users - GateKeeper",
};

type Props = {
  params: Promise<{
    organizationId: string;
  }>;
};

export default async function UsersPage({ params }: Props) {
  const { organizationId } = await params;
  const organizationName =
    (await cookies()).get("organization")?.value || "Organization Detail";

  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: "/dashboard" },
            { name: organizationName, path: `/dashboard/${organizationId}` },
            { name: "Users", path: `/dashboard/${organizationId}/users` },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <UsersTab />
      </main>
    </>
  );
}
