"use client";

import { Building, Paintbrush } from "lucide-react";
import { useState } from "react";
import { usePathname, useRouter } from "next/navigation";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "../ui/dialog";
import { SidebarProvider } from "../ui/sidebar";

import { SettingsSidebar } from "./settings-sidebar";

import { AppearanceSection } from "./appearance-section";

type Props = {
  isOpened: boolean;
  onOpenChange: (value: boolean) => void;
};

export type NavProps = {
  name: string;
  icon: React.ElementType;
  component: React.ElementType;
};

export function SettingsDialog({ isOpened, onOpenChange }: Props) {
  const nav: NavProps[] = [
    { name: "Appearance", icon: Paintbrush, component: AppearanceSection },
  ];

  const [selectedSection, setSelectedSection] = useState<NavProps | null>(
    nav[0],
  );

  const router = useRouter();
  const pathname = usePathname();

  return (
    <Dialog
      open={isOpened}
      onOpenChange={(value) => {
        if (!value) {
          router.push(pathname);
        }

        onOpenChange(value);
      }}
    >
      <DialogContent className="overflow-hidden p-0 md:max-h-[700px] md:max-w-[700px] lg:max-w-[1366px] w-[90%]">
        <DialogTitle className="sr-only">Settings</DialogTitle>
        <DialogDescription className="sr-only">
          Customize your settings here.
        </DialogDescription>

        <SidebarProvider className="items-start">
          <SettingsSidebar
            nav={nav}
            selectSection={setSelectedSection}
            selectedSection={selectedSection}
          />

          {selectedSection?.component ? <selectedSection.component /> : <></>}
        </SidebarProvider>
      </DialogContent>
    </Dialog>
  );
}
