"use client";

import Link from "next/link";
import { Metadata } from "next";
import { ChevronLeft } from "lucide-react";

import { DashboardHeader } from "@/components/dashboard-header";
import { useTenantsContext } from "@/app/dashboard/(contexts)/tenants-context-provider";
import { CreateApplicationForm } from "./create-application-form";

export function CreateApplicationContent({}) {
  const { selectedTenant } = useTenantsContext();

  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: `/dashboard` },
            {
              name: selectedTenant?.name || "Tenant",
              path: `/dashboard/${selectedTenant?.id}`,
            },
            {
              name: "Applications",
              path: `/dashboard/${selectedTenant?.id}/application`,
            },
            {
              name: "Create Application",
              path: `/dashboard/${selectedTenant?.id}/application/create-application`,
            },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <Link
          href={`/dashboard/${selectedTenant?.id}?tab=applications`}
          className="text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:dark:text-gray-500 hover:text-gray-800 hover:underline"
        >
          <ChevronLeft size={24} />
          Go back to applications list
        </Link>

        <h2 className="text-3xl font-bold tracking-tight">
          Create Application
        </h2>

        <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
          Here you can create a new application. Fill in the form below and
          click the &quot;Create Application&quot; button to create a new
          application.
        </span>

        <CreateApplicationForm />
      </main>
    </>
  );
}
