import { useState } from "react";

import { Separator } from "@/components/ui/separator";
import {
  Card,
  CardContent,
  CardDescription,
  CardTitle,
} from "@/components/ui/card";

import { DeleteOrganizationDialog } from "./delete-organization-dialog";
import { useOrganizationsContext } from "@/app/dashboard/(contexts)/organizations-context-provider";

export function DeleteOrganizationCard() {
  const { selectedOrganization } = useOrganizationsContext();

  const [openDeleteOrganizationDialog, setOpenDeleteOrganizationDialog] =
    useState(false);

  return (
    <Card className="border-destructive">
      <CardTitle className="px-4">Danger Zone</CardTitle>

      <Separator className="bg-destructive" />

      <CardContent className="flex px-4 gap-4 justify-between items-start">
        <div className="flex flex-col gap-4">
          <span className="font-semibold">Delete this organization</span>

          <CardDescription>
            Once you delete an organization, it will be sent to the trash
            section. There you will be able to reactivate it again.
          </CardDescription>
        </div>

        <DeleteOrganizationDialog
          isModalOpened={openDeleteOrganizationDialog}
          onOpenChange={setOpenDeleteOrganizationDialog}
          organization={selectedOrganization || null}
        />
      </CardContent>
    </Card>
  );
}
