import { Metadata } from "next";

import { Separator } from "@/components/ui/separator";
import { DashboardHeader } from "@/components/dashboard-header";

import { TenantsTable } from "./(components)/tenants-table";

export const metadata: Metadata = {
  title: "Tenants - GateKeeper",
};

export default function TenantsPage() {
  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: "/dashboard" },
            { name: "Tenants", path: "/dashboard/tenants" },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <h2 className="text-3xl font-bold tracking-tight">Tenants</h2>

        <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
          Here you can manage the tenants registered in the application,
          including adding new tenants, editing existing ones, and
          removing those that are no longer needed.
        </span>

        <Separator className="my-4" />

        <section className="flex flex-col gap-4">
          <TenantsTable />
        </section>
      </main>
    </>
  );
}
