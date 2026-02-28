import { Breadcrumbs } from "@/components/dashboard-header/bread-crumbs";
import { Separator } from "@/components/ui/separator";

import { OrganizationsTable } from "./organizations-table";
import { OrganizationsPages } from "..";

type Props = {
  setPage: (page: OrganizationsPages) => void;
};

export function DefaultPage({ setPage }: Props) {
  return (
    <main className="flex flex-col p-4">
      <Breadcrumbs
        items={[{ name: "Settings" }, { name: "Organizations" }]}
        disableSideBar
      />
      <h2 className="text-3xl font-bold tracking-tight mt-4">Organizations</h2>

      <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
        Here you can manage the organizations registered in the application,
        including adding new organizations, editing existing ones, and removing
        those that are no longer needed.
      </span>

      <Separator className="my-4" />

      <section className="flex flex-col gap-4">
        <OrganizationsTable setPage={setPage} />
      </section>
    </main>
  );
}
