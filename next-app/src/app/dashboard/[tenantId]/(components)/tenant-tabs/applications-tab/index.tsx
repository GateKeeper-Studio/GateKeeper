"use client";

import { ErrorAlert } from "@/components/error-alert";
import { SectionTitle } from "@/components/section-title";

import {
  ApplicationsTable,
  type ApplicationTableItem,
} from "./applications-table";
import { useApplicationsSWR } from "@/services/dashboard/use-applications-swr";
import { useParams } from "next/navigation";

export function ApplicationsTab() {
  const tenantId = useParams().tenantId as string;

  const { data, error, isLoading, mutate } = useApplicationsSWR(
    { tenantId: tenantId },
    { accessToken: "" },
  );

  if (error) {
    return (
      <div className="flex m-4 w-full">
        <ErrorAlert
          message={
            error.response?.data.message ||
            "Failed on trying to fetch applications"
          }
          title={error.response?.data.title || "An Error Occurred"}
        />
      </div>
    );
  }

  return (
    <section className="flex flex-col gap-4 w-full">
      <SectionTitle>Applications</SectionTitle>

      <span className="text-sm tracking-tight text-gray-600 dark:text-gray-300">
        Each application can have its own users, roles and permissions, allowing
        you to manage access and data tenant effectively.
      </span>

      <ApplicationsTable
        items={data || []}
        setItems={(items: ApplicationTableItem[]) =>
          mutate(items, { revalidate: false })
        }
        isLoading={isLoading}
      />
    </section>
  );
}
