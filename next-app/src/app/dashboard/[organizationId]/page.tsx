import { Breadcrumbs } from "@/components/bread-crumbs";
import { ErrorAlert } from "@/components/error-alert";
import { getOrganizationByIdService } from "@/services/dashboard/get-organization-by-id";
import { Metadata } from "next";

type Props = {
  params: Promise<{
    organizationId: string;
  }>;
};

export const metadata: Metadata = {
  title: "Organizations - GateKeeper",
};

export default async function OrganizationPage({ params }: Props) {
  const { organizationId } = await params;

  const [organizationData, err] = await getOrganizationByIdService({ organizationId }, { accessToken: "" });

  if (err) {
    return (
      <div className="flex m-4 w-full">
        <ErrorAlert
          message={
            err.response?.data.message ||
            "Failed on trying to fetch organization"
          }
          title={err.response?.data.title || "An Error Occurred"}
        />
      </div>
    );
  }

  return (
    <>
      <Breadcrumbs
        items={[
          { name: "Dashboard", path: "/dashboard" },
          {
            name: organizationData?.name || "Organization Details",
            path: `/dashboard/${organizationId}`,
          },
          {
            name: "Applications",
            path: `/dashboard/${organizationId}/application`,
          },
        ]}
      />

      <main className="flex flex-col p-4">
        <h2 className="text-3xl font-bold tracking-tight">{organizationData?.name}</h2>

        <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
          {organizationData?.description || "No description provided."}
        </span>

        <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
          Applications are the projects you have created. You can add, edit, and
          delete them here.
        </span>
      </main>
    </>
  );
}
