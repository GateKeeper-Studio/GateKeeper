import Link from "next/link";
import { cookies } from "next/headers";
import { ChevronLeft } from "lucide-react";

import { UserDetailContent } from "./(components)/user-detail-content";

import { DashboardHeader } from "@/components/dashboard-header";
import { getTenantUserByIdService } from "@/services/dashboard/get-tenant-user-by-id";

type Props = {
  params: Promise<{
    organizationId: string;
    applicationId: string;
    userId: string;
  }>;
};

export async function generateMetadata({ params }: Props) {
  const { organizationId, userId } = await params;

  const organizationName =
    (await cookies()).get("organization")?.value || "Organization Detail";

  const [user, err] = await getTenantUserByIdService(
    { userId, organizationId },
    { accessToken: "" },
  );

  if (err) {
    return {
      title: "User - GateKeeper",
    };
  }

  return {
    title: `${user?.displayName} - User - GateKeeper`,
  };
}

export default async function UserDetailAndEditPage({ params }: Props) {
  const { organizationId, userId } = await params;
  const organizationName =
    (await cookies()).get("organization")?.value || "Organization Detail";

  const [data] = await getTenantUserByIdService(
    {
      organizationId,
      userId,
    },
    { accessToken: "" },
  );

  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: "/dashboard" },
            { name: organizationName, path: `/dashboard/${organizationId}` },
            {
              name: "Users",
              path: `/dashboard/${organizationId}/users`,
            },
            {
              name: data?.displayName || "User Detail",
              path: `/dashboard/${organizationId}/users/${userId}`,
            },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <Link
          href={`/dashboard/${organizationId}/users`}
          className="text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:text-gray-800  hover:underline"
        >
          <ChevronLeft size={24} />
          Go back to users
        </Link>

        <UserDetailContent user={data} />
      </main>
    </>
  );
}
