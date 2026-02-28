import { Separator } from "@/components/ui/separator";
import { Breadcrumbs } from "@/components/dashboard-header/bread-crumbs";

import { ThemeSwitcher } from "./theme-switcher";
import { ColorSwitcher } from "./color-switcher";

export function AppearanceSection() {
  return (
    <main className="flex flex-col p-4 md:max-h-[700px] overflow-auto overflow-x-hidden">
      <Breadcrumbs
        items={[{ name: "Settings" }, { name: "Appearance" }]}
        disableSideBar
      />
      <h2 className="text-3xl font-bold tracking-tight mt-4">Appearance</h2>

      <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
        Here you can manage the appearance settings of the application,
        including theme selection, font size adjustment, and layout options.
      </span>

      <Separator className="my-4" />

      <section className="flex flex-col gap-2">
        <h3 className="text-2xl font-semibold tracking-tight">Theming</h3>
        <span className="text-sm tracking-tight text-gray-600 dark:text-gray-300">
          Here you can manage the theme settings of the application.
        </span>

        <ThemeSwitcher />
      </section>

      <Separator className="my-4" />

      <section className="flex flex-col gap-2">
        <h3 className="text-2xl font-semibold tracking-tight">Primary Color</h3>
        <span className="text-sm tracking-tight text-gray-600 dark:text-gray-300">
          Here you can manage the theme settings of the application.
        </span>

        <ColorSwitcher />
      </section>
    </main>
  );
}
