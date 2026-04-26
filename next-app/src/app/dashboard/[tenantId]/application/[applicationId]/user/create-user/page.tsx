import Link from "next/link";
import { Metadata } from "next";
import { cookies } from "next/headers";
import { ChevronLeft } from "lucide-react";

import { UserDetailForm } from "./(components)/user-detail-form";
import { DashboardHeader } from "@/components/dashboard-header";

type Props = {
  params: Promise<{
    tenantId: string;
    applicationId: string;
    userId: string;
  }>;
};

export const metadata: Metadata = {
  title: "Add User - GateKeeper",
};

export default async function CreateUserPage({ params }: Props) {
  const { tenantId, applicationId, userId } = await params;

  const tenantName =
    (await cookies()).get("tenant")?.value || "Tenant Detail";
  const applicationName =
    (await cookies()).get("application")?.value || "Application Detail";

  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: "/dashboard" },
            {
              name: tenantName,
              path: `/dashboard/${tenantId}`,
            },
            {
              name: "Application",
              path: `/dashboard/${tenantId}/application`,
            },
            {
              name: applicationName || "Application Detail",
              path: `/dashboard/${tenantId}/application/${applicationId}`,
            },
            {
              name: "Users",
              path: `/dashboard/${tenantId}/application/${applicationId}?tab=users`,
            },
            {
              name: userId,
              path: `/dashboard/${tenantId}/application/${applicationId}/user/${userId}`,
            },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <Link
          href={`/dashboard/${tenantId}/application/${applicationId}?tab=users`}
          className="text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:text-gray-800  hover:underline"
        >
          <ChevronLeft size={24} />
          Go back to application detail
        </Link>

        <UserDetailForm />
      </main>
    </>
  );
}
