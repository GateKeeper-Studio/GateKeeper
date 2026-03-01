import { Metadata } from "next";

import { Separator } from "@/components/ui/separator";
import { DashboardHeader } from "@/components/dashboard-header";

import { OrganizationsTable } from "./(components)/organizations-table";

export const metadata: Metadata = {
  title: "Organizations - GateKeeper",
};

export default function OrganizationsPage() {
  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: "/dashboard" },
            { name: "Organizations", path: "/dashboard/organizations" },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <h2 className="text-3xl font-bold tracking-tight">Organizations</h2>

        <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
          Here you can manage the organizations registered in the application,
          including adding new organizations, editing existing ones, and
          removing those that are no longer needed.
        </span>

        <Separator className="my-4" />

        <section className="flex flex-col gap-4">
          <OrganizationsTable />
        </section>
      </main>
    </>
  );
}
