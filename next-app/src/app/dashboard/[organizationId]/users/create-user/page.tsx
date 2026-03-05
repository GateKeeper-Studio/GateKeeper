import Link from "next/link";
import { Metadata } from "next";
import { cookies } from "next/headers";
import { ChevronLeft } from "lucide-react";

import { DashboardHeader } from "@/components/dashboard-header";
import { UserDetailForm } from "@/app/dashboard/[organizationId]/application/[applicationId]/user/create-user/(components)/user-detail-form";

export const metadata: Metadata = {
  title: "Add User - GateKeeper",
};

type Props = {
  params: Promise<{
    organizationId: string;
  }>;
};

export default async function CreateUserPage({ params }: Props) {
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
            {
              name: "Add User",
              path: `/dashboard/${organizationId}/users/create-user`,
            },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <Link
          href={`/dashboard/${organizationId}/users`}
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
