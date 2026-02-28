import { ChevronLeft } from "lucide-react";
import { useSearchParams } from "next/navigation";

import { Breadcrumbs } from "@/components/dashboard-header/bread-crumbs";

import { OrganizationsPages } from "..";
import { EditOrganizationForm } from "./edit-organization-form";

import { useOrganizationByIdSWR } from "@/services/settings/use-organization-by-id-swr";
import { Skeleton } from "@/components/ui/skeleton";

type Props = {
  setPage: (page: OrganizationsPages) => void;
};

export function EditOrganization({ setPage }: Props) {
  const searchParams = useSearchParams();

  const { data: organization, isLoading } = useOrganizationByIdSWR(
    { id: searchParams.get("organizationId") || "" },
    { accessToken: "fake-token" },
  );

  return (
    <main className="flex flex-col p-4">
      <Breadcrumbs
        items={[
          { name: "Settings" },
          { name: "Organizations" },
          { name: "Add Organization" },
        ]}
        disableSideBar
      />

      <button
        type="button"
        onClick={() => setPage("default")}
        className="mt-4 w-fit text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:dark:text-gray-500 hover:text-gray-800 hover:underline"
      >
        <ChevronLeft size={24} />
        Go back to organizations list
      </button>

      <h2 className="text-3xl font-bold tracking-tight">Update Organization</h2>

      <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
        Here you can add a new organization. Fill in the form below and click
        the &quot;Add Organization&quot; button to create a new organization.
      </span>

      {isLoading || !organization ? (
        <div className="mt-4 max-w-[700px] flex flex-col gap-4">
          <div className="flex flex-col gap-1">
            <Skeleton className="w-[56px] h-[16px]" />
            <Skeleton className="w-full h-[40px]" />
          </div>

          <div className="flex flex-col gap-1">
            <Skeleton className="w-[56px] h-[16px]" />
            <Skeleton className="w-full h-[56px]" />
          </div>
        </div>
      ) : (
        <EditOrganizationForm setPage={setPage} organization={organization} />
      )}
    </main>
  );
}
