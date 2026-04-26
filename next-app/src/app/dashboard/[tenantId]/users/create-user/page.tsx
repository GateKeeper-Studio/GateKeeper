import Link from "next/link";
import { Metadata } from "next";
import { cookies } from "next/headers";
import { ChevronLeft } from "lucide-react";

import { DashboardHeader } from "@/components/dashboard-header";
import { UserDetailForm } from "@/app/dashboard/[tenantId]/application/[applicationId]/user/create-user/(components)/user-detail-form";

export const metadata: Metadata = {
  title: "Add User - GateKeeper",
};

type Props = {
  params: Promise<{
    tenantId: string;
  }>;
};

export default async function CreateUserPage({ params }: Props) {
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
            {
              name: "Add User",
              path: `/dashboard/${tenantId}/users/create-user`,
            },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <Link
          href={`/dashboard/${tenantId}/users`}
          className="text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:text-gray-800 hover:underline"
        >
          <ChevronLeft size={24} />
          Go back to users
        </Link>

        <UserDetailForm />
      </main>
    </>
  );
}
