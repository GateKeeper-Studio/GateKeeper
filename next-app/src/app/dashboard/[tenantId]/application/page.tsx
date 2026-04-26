import Link from "next/link";
import { Metadata } from "next";
import { cookies } from "next/headers";

import { buttonVariants } from "@/components/ui/button";
import { Tabs, TabsTrigger, TabsList, TabsContent } from "@/components/ui/tabs";

import { cn } from "@/lib/utils";
import { DashboardHeader } from "@/components/dashboard-header";
import { ApplicationCard } from "./(components)/application-card";

type Props = {
  params: Promise<{
    tenantId: string;
  }>;
};

export const metadata: Metadata = {
  title: "Applications - GateKeeper",
};

export default async function ApplicationsPage({ params }: Props) {
  const { tenantId } = await params;

  const tenantName =
    (await cookies()).get("tenant")?.value || "Tenant Detail";

  return (
    <>
      <DashboardHeader
        breadcrumbs={{
          items: [
            { name: "Dashboard", path: "/dashboard" },
            {
              name: tenantName,
              path: `/dashboard/${tenantId}`,
            },
            {
              name: "Applications",
              path: `/dashboard/${tenantId}/application`,
            },
          ],
        }}
      />

      <main className="flex flex-col p-4">
        <h2 className="text-3xl font-bold tracking-tight">Applications</h2>

        <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
          Applications are the projects you have created. You can add, edit, and
          delete them here.
        </span>

        <Tabs defaultValue="overview" className="mt-4">
          <div className="flex justify-between items-center gap-4">
            <TabsList>
              <TabsTrigger value="overview">Overview</TabsTrigger>
            </TabsList>

            <Link
              href={`/dashboard/${tenantId}/application/create-application`}
              className={cn(
                "float-right w-fit",
                buttonVariants({ variant: "default" }),
              )}
            >
              New Application
            </Link>
          </div>

          <TabsContent value="overview" className="flex flex-1 flex-wrap gap-3">
            <ApplicationCard />
          </TabsContent>
        </Tabs>
      </main>
    </>
  );
}
