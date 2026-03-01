import { SectionTitle } from "@/components/section-title";

import { DeleteOrganizationCard } from "./delete-organization-card";

export function SettingsTab() {
  return (
    <section className="flex flex-col gap-4 w-full">
      <SectionTitle>Settings</SectionTitle>

      <span className="text-sm tracking-tight text-gray-600 dark:text-gray-300">
        Here you can manage the settings of this organization.
      </span>

      <DeleteOrganizationCard />
    </section>
  );
}
