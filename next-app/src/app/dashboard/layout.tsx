import { PropsWithChildren } from "react";

import { Toaster } from "@/components/ui/sonner";
import { SidebarProvider } from "@/components/ui/sidebar";
import { DashboardSidebar } from "@/components/dashboard-sidebar";
import { OrganizationsContextProvider } from "./(contexts)/organizations-context-provider";

type Props = PropsWithChildren<object>;

export default function Layout({ children }: Props) {
  return (
    <div className="flex h-screen w-screen">
      <OrganizationsContextProvider>
        <SidebarProvider>
          <DashboardSidebar />

          <div className="flex-1">
            <Toaster richColors />

            {children}
          </div>
        </SidebarProvider>
      </OrganizationsContextProvider>
    </div>
  );
}
