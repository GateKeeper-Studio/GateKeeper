"use client";

import { useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";

import { IApplication } from "@/services/dashboard/get-application-by-id";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

import { Roles } from "./roles";
import { Users } from "./users";
import { Overview } from "./overview";
import { Providers } from "./providers";
import { ChartColumn, IdCard, UserRoundKey, Users2 } from "lucide-react";

type Props = {
  application: IApplication | null;
  organizationId: string;
};

export function ApplicationTabs({ application, organizationId }: Props) {
  const searchParams = useSearchParams();
  const tab = searchParams.get("tab");
  const router = useRouter();

  const [currentTab, setCurrentTab] = useState<string>(
    (tab as string) || "overview",
  );

  useEffect(() => {
    if (["overview", "users", "roles", "providers"].includes(tab as string)) {
      setCurrentTab(tab as "overview" | "users" | "roles" | "providers");
    }
  }, [tab]);

  return (
    <Tabs
      className="mt-4"
      value={currentTab}
      onValueChange={(value) => {
        setCurrentTab(value);
        router.push(
          `/dashboard/${organizationId}/application/${application?.id}?tab=${value}`,
        );
      }}
    >
      <TabsList>
        <TabsTrigger value="overview">
          <ChartColumn />
          Overview
        </TabsTrigger>
        <TabsTrigger value="users">
          <Users2 />
          Users
        </TabsTrigger>
        <TabsTrigger value="roles">
          <UserRoundKey />
          Roles
        </TabsTrigger>
        <TabsTrigger value="providers">
          <IdCard />
          Providers
        </TabsTrigger>
      </TabsList>

      <TabsContent value="overview">
        <Overview application={application} />
      </TabsContent>

      <TabsContent value="users">
        <Users application={application} />
      </TabsContent>

      <TabsContent value="roles">
        <Roles application={application} />
      </TabsContent>

      <TabsContent value="providers">
        <Providers application={application} />
      </TabsContent>
    </Tabs>
  );
}
