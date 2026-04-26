import { SectionTitle } from "@/components/section-title";

import { DeleteTenantCard } from "./delete-tenant-card";

export function SettingsTab() {
  return (
    <section className="flex flex-col gap-4 w-full">
      <SectionTitle>Settings</SectionTitle>

      <span className="text-sm tracking-tight text-gray-600 dark:text-gray-300">
        Here you can manage the settings of this tenant.
      </span>

      <DeleteTenantCard />
    </section>
  );
}
