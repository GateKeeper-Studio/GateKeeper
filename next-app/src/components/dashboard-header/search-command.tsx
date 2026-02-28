"use client";

import { useState } from "react";
import {
  BookDashed,
  Box,
  CalculatorIcon,
  CalendarIcon,
  CreditCardIcon,
  DraftingCompass,
  LayoutPanelLeft,
  ListTree,
  Search,
  SettingsIcon,
  SmileIcon,
  UserIcon,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from "@/components/ui/command";
import { useRouter } from "next/navigation";

export function SearchCommand() {
  const router = useRouter();
  const [open, setOpen] = useState(false);

  const sidebarItems = [
    {
      name: "Applications",
      icon: LayoutPanelLeft,
      path: "/dashboard/applications",
    },
  ];

  return (
    <div className="flex flex-col gap-4 ml-auto">
      <Button onClick={() => setOpen(true)} variant="outline" className="w-fit">
        <Search className="size-4 mr-2" />

        <span className="font-medium text-muted-foreground">
          Search for something...
        </span>
      </Button>

      <CommandDialog open={open} onOpenChange={setOpen}>
        <Command>
          <CommandInput placeholder="Type a command or search..." />
          <CommandList>
            <CommandEmpty>No results found.</CommandEmpty>
            <CommandGroup heading="Navigation">
              {sidebarItems.map((item) => (
                <CommandItem
                  onSelect={() => {
                    router.push(item.path);
                    setOpen(false);
                  }}
                  key={item.name}
                >
                  <item.icon />
                  <span>{item.name}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </CommandDialog>
    </div>
  );
}
