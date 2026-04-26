"use client";

import Link from "next/link";
import { ChevronLeft } from "lucide-react";

import { CreateTenantForm } from "./create-tenant-form";

export function CreateTenantContent() {
  return (
    <main className="flex flex-col p-4">
      <Link
        href="/dashboard/tenants"
        className="w-fit text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:dark:text-gray-500 hover:text-gray-800 hover:underline"
      >
        <ChevronLeft size={24} />
        Go back to tenants list
      </Link>

      <h2 className="text-3xl font-bold tracking-tight">Add Tenant</h2>

      <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
        Here you can add a new tenant. Fill in the form below and click
        the &quot;Add Tenant&quot; button to create a new tenant.
      </span>

      <CreateTenantForm />
    </main>
  );
}
