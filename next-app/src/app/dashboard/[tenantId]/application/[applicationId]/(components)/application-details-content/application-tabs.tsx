"use client";

import { useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";

import { IApplication } from "@/services/dashboard/get-application-by-id";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

import { Roles } from "./roles";
import { Overview } from "./overview";
import { Providers } from "./providers";
import {
  ChartColumn,
  GroupIcon,
  IdCard,
  UserRoundKey,
  Users2,
} from "lucide-react";
import { UsersTab } from "./users-tab";
import { Separator } from "@/components/ui/separator";

type Props = {
  application: IApplication | null;
  tenantId: string;
};

export function ApplicationTabs({ application, tenantId }: Props) {
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
          `/dashboard/${tenantId}/application/${application?.id}?tab=${value}`,
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

        <TabsTrigger value="groups">
          <GroupIcon />
          Groups
        </TabsTrigger>

        <TabsTrigger value="providers">
          <IdCard />
          OAuth Providers
        </TabsTrigger>
      </TabsList>

      <Separator className="my-2" />

      <TabsContent value="overview">
        <Overview application={application} />
      </TabsContent>

      <TabsContent value="users">
        <UsersTab />
      </TabsContent>

      <TabsContent value="roles">
        <Roles />
      </TabsContent>

      <TabsContent value="providers">
        <Providers application={application} />
      </TabsContent>
    </Tabs>
  );
}
