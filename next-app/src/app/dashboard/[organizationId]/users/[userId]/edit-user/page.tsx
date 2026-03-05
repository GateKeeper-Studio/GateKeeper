import Link from "next/link";
import { cookies } from "next/headers";
import { ChevronLeft } from "lucide-react";

import { DashboardHeader } from "@/components/dashboard-header";
import { EditUserForm } from "@/app/dashboard/[organizationId]/application/[applicationId]/user/[userId]/edit-user/(components)/edit-user-form";
import { getTenantUserByIdService } from "@/services/dashboard/get-tenant-user-by-id";

type Props = {
  params: Promise<{
    organizationId: string;
    userId: string;
  }>;
};

export async function generateMetadata({ params }: Props) {
  const { organizationId, userId } = await params;

  const [user, err] = await getTenantUserByIdService(
    { userId, organizationId },
    { accessToken: "" },
  );

  if (err) {
    return {
      title: "Edit User - GateKeeper",
    };
  }

  return {
    title: `Edit ${user?.displayName} - User - GateKeeper`,
  };
}

export default async function EditUserPage({ params }: Props) {
  const { organizationId, userId } = await params;
  const organizationName =
    (await cookies()).get("organization")?.value || "Organization Detail";

  const [data, err] = await getTenantUserByIdService(
    { organizationId, userId },
    { accessToken: "" },
  );

  if (err) {
    return <div>Failed to fetch user</div>;
  }

  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: "/dashboard" },
            { name: organizationName, path: `/dashboard/${organizationId}` },
            { name: "Users", path: `/dashboard/${organizationId}/users` },
            {
              name: data?.displayName || "User Detail",
              path: `/dashboard/${organizationId}/users/${userId}`,
            },
            {
              name: "Edit User",
              path: `/dashboard/${organizationId}/users/${userId}/edit-user`,
            },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <Link
          href={`/dashboard/${organizationId}/users/${userId}`}
          className="text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:dark:text-gray-500 hover:text-gray-800 hover:underline"
        >
          <ChevronLeft size={24} />
          Go back to user details
        </Link>

        <h2 className="text-3xl font-bold tracking-tight">Edit User</h2>

        <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
          Here you can edit the user. Fill in the form below and click the
          &quot;Apply Changes&quot; button to save.
        </span>

        <EditUserForm user={data} />
      </main>
    </>
  );
}
