"use client";

import { toast } from "sonner";
import { useState } from "react";

import { Button, buttonVariants } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

import { Organization } from "@/services/dashboard/use-organizations-swr";
import { deleteOrganizationApi } from "@/services/settings/delete-organization";

type Props = {
  isOpened: boolean;
  onOpenChange: (value: boolean) => void;
  organization: Organization | null;
  removeOrganization: (organization: Organization) => void;
};

export function DeleteOrganizationDialog({
  isOpened,
  onOpenChange,
  organization,
  removeOrganization,
}: Props) {
  const [isLoading, setIsLoading] = useState(false);

  async function handler() {
    if (!organization) {
      console.error("Organization is not defined");
      toast.error("Organization is not defined");
      return;
    }

    setIsLoading(true);

    const [err] = await deleteOrganizationApi(
      { organizationId: organization.id },
      { accessToken: "fake-token" },
    );

    if (err) {
      console.error(err);
      toast.error("Failed to delete organization");
      setIsLoading(false);
      return;
    }

    removeOrganization(organization);

    setIsLoading(false);

    toast.success("Organization deleted successfully!");

    onOpenChange(false);
  }

  return (
    <Dialog open={isOpened} onOpenChange={(value) => onOpenChange(value)}>
      <DialogContent className="sm:max-w-[450px]">
        <DialogHeader>
          <DialogTitle>Delete Organization</DialogTitle>

          <DialogDescription>
            On deleting this organization, it will be permanently removed and
            all the applications created under it will also be deleted. Are you
            sure you want to proceed?
          </DialogDescription>
        </DialogHeader>

        <DialogFooter>
          <DialogClose className={buttonVariants({ variant: "outline" })}>
            Cancel
          </DialogClose>

          <Button type="submit" onClick={handler} variant="destructive">
            {isLoading ? "Deleting..." : "Delete"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
