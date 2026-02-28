import { Metadata } from "next";

import { Separator } from "@/components/ui/separator";

import { OrganizationsList } from "./(components)/organizations-list";
import { DashboardHeader } from "@/components/dashboard-header";

export const metadata: Metadata = {
  title: "Organizations - GateKeeper",
};

export default function DashboardPage() {
  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [{ name: "Dashboard", path: "/dashboard" }],
        }}
      />

      <main className="flex flex-col p-4">
        <h2 className="text-3xl font-bold tracking-tight">Dashboard</h2>

        <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
          Welcome to your dashboard! Here you can manage your applications.
        </span>

        <Separator className="my-4" />

        <section className="flex flex-col gap-4">
          <h3 className="text-2xl font-bold tracking-tight">Organizations</h3>

          <div className="flex flex-1 flex-wrap gap-3 w-full">
            <OrganizationsList />
          </div>
        </section>
      </main>
    </>
  );
}
