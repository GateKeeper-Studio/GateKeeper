"use client";

import {
  BookDashed,
  ChartColumn,
  DraftingCompass,
  LayoutPanelLeft,
  ListTree,
  Radar,
  Settings,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useSearchParams, useRouter } from "next/navigation";

import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

import { SettingsTab } from "./settings-tab";

import { ApplicationsTab } from "./applications-tab";
import { useTenantsContext } from "@/app/dashboard/(contexts)/tenants-context-provider";

type Props = {};

export const tenantTabs = [
  { key: "applications", title: "Applications" },
  { key: "settings", title: "Settings" },
];

export function TenantTabs({}: Props) {
  const { selectedTenant } = useTenantsContext();
  const router = useRouter();
  const searchParams = useSearchParams();
  const tab = searchParams.get("tab");

  const [currentTab, setCurrentTab] = useState<string>(
    (tab as string) || tenantTabs[0].key,
  );

  useEffect(() => {
    if (tenantTabs.some((tabItem) => tabItem.key === (tab as string))) {
      setCurrentTab(tab as "applications" | "settings");
    }
  }, [tab]);

  return (
    <Tabs
      className="mt-4"
      value={currentTab}
      onValueChange={(value) => {
        setCurrentTab(value);
        router.push(`/dashboard/${selectedTenant?.id}?tab=${value}`);
      }}
    >
      <div className="flex justify-between items-center gap-4">
        <TabsList>
          <TabsTrigger value="applications">
            <LayoutPanelLeft /> Applications
          </TabsTrigger>

          <TabsTrigger value="settings">
            <Settings /> Settings
          </TabsTrigger>
        </TabsList>
      </div>

      <Separator className="my-2" />

      <TabsContent value="applications" className="flex flex-1 flex-wrap gap-3">
        <ApplicationsTab />
      </TabsContent>

      <TabsContent value="settings" className="flex flex-1 flex-wrap gap-3">
        <SettingsTab />
      </TabsContent>
    </Tabs>
  );
}
