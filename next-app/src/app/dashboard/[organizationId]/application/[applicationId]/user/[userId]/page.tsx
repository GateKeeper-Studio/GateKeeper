import Link from "next/link";
import { ChevronLeft } from "lucide-react";

import { Breadcrumbs } from "@/components/bread-crumbs";
import { UserDetailForm } from "./(components)/user-detail-form";

import { getApplicationUserByIdService } from "@/services/dashboard/get-application-user-by-id";
import { cookies } from "next/headers";

type Props = {
  params: Promise<{
    organizationId: string;
    applicationId: string;
    userId: string;
  }>;
};

export async function generateMetadata({ params }: Props) {
  const { applicationId, organizationId, userId } = await params;

  const organizationName = (await cookies()).get("organization")?.value || "Organization Detail";

  const [user, err] = await getApplicationUserByIdService(
    { applicationId, userId, organizationId },
    { accessToken: "" }
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
  const { organizationId, applicationId, userId } = await params;
  const organizationName = (await cookies()).get("organization")?.value || "Organization Detail";

  const [data] = await getApplicationUserByIdService(
    {
      applicationId,
      organizationId,
      userId,
    },
    { accessToken: "" }
  );

  return (
    <>
      <Breadcrumbs
        items={[
          { name: "Dashboard", path: "/dashboard" },
          { name: organizationName, path: `/dashboard/${organizationId}` },
          {
            name: "Application",
            path: `/dashboard/${organizationId}/application`,
          },
          {
            name: data?.applicationName || "Application Detail",
            path: `/dashboard/${organizationId}/application/${applicationId}`,
          },
          {
            name: "Users",
            path: `/dashboard/${organizationId}/application/${applicationId}?tab=users`,
          },
          {
            name: data?.displayName || "User Detail",
            path: `/dashboard/${organizationId}/application/${applicationId}/user/${userId}`,
          },
        ]}
      />

      <main className="flex flex-col p-4">
        <Link
          href={`/dashboard/${organizationId}/application/${applicationId}?tab=users`}
          className="text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:text-gray-800  hover:underline"
        >
          <ChevronLeft size={24} />
          Go back to application detail
        </Link>

        <UserDetailForm user={data} />
      </main>
    </>
  );
}
