import { useState } from "react";

import { Separator } from "@/components/ui/separator";
import {
  Card,
  CardContent,
  CardDescription,
  CardTitle,
} from "@/components/ui/card";

import { DeleteTenantDialog } from "./delete-tenant-dialog";
import { useTenantsContext } from "@/app/dashboard/(contexts)/tenants-context-provider";

export function DeleteTenantCard() {
  const { selectedTenant } = useTenantsContext();

  const [openDeleteTenantDialog, setOpenDeleteTenantDialog] =
    useState(false);

  return (
    <Card className="border-destructive">
      <CardTitle className="px-4">Danger Zone</CardTitle>

      <Separator className="bg-destructive" />

      <CardContent className="flex px-4 gap-4 justify-between items-start">
        <div className="flex flex-col gap-4">
          <span className="font-semibold">Delete this tenant</span>

          <CardDescription>
            Once you delete an tenant, it will be sent to the trash
            section. There you will be able to reactivate it again.
          </CardDescription>
        </div>

        <DeleteTenantDialog
          isModalOpened={openDeleteTenantDialog}
          onOpenChange={setOpenDeleteTenantDialog}
          tenant={selectedTenant || null}
        />
      </CardContent>
    </Card>
  );
}
