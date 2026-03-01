"use client";

import Link from "next/link";
import { ChevronLeft } from "lucide-react";

import { EditOrganizationForm } from "./edit-organization-form";

type Props = {
  organization: {
    id: string;
    name: string;
    description: string;
  };
};

export function EditOrganizationContent({ organization }: Props) {
  return (
    <main className="flex flex-col p-4">
      <Link
        href="/dashboard/organizations"
        className="w-fit text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:dark:text-gray-500 hover:text-gray-800 hover:underline"
      >
        <ChevronLeft size={24} />
        Go back to organizations list
      </Link>

      <h2 className="text-3xl font-bold tracking-tight">Update Organization</h2>

      <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
        Here you can update the organization details. Fill in the form below and
        click the &quot;Edit Organization&quot; button to save the changes.
      </span>

      <EditOrganizationForm organization={organization} />
    </main>
  );
}
