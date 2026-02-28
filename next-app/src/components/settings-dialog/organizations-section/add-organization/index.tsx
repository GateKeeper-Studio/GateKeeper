import { ChevronLeft } from "lucide-react";

import { Breadcrumbs } from "@/components/dashboard-header/bread-crumbs";

import { AddOrganizationForm } from "./add-organization-form";
import { OrganizationsPages } from "..";

type Props = {
  setPage: (page: OrganizationsPages) => void;
};

export function AddOrganization({ setPage }: Props) {
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

      <h2 className="text-3xl font-bold tracking-tight">Add Organization</h2>

      <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
        Here you can add a new organization. Fill in the form below and click
        the &quot;Add Organization&quot; button to create a new organization.
      </span>

      <AddOrganizationForm setPage={setPage} />
    </main>
  );
}
