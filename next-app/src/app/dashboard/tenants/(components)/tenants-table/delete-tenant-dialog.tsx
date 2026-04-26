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

import { Tenant } from "@/services/dashboard/use-tenants-swr";
import { deleteTenantApi } from "@/services/settings/delete-tenant";

type Props = {
  isOpened: boolean;
  onOpenChange: (value: boolean) => void;
  tenant: Tenant | null;
  removeTenant: (tenant: Tenant) => void;
};

export function DeleteTenantDialog({
  isOpened,
  onOpenChange,
  tenant,
  removeTenant,
}: Props) {
  const [isLoading, setIsLoading] = useState(false);

  async function handler() {
    if (!tenant) {
      console.error("Tenant is not defined");
      toast.error("Tenant is not defined");
      return;
    }

    setIsLoading(true);

    const [err] = await deleteTenantApi(
      { tenantId: tenant.id },
      { accessToken: "fake-token" },
    );

    if (err) {
      console.error(err);
      toast.error("Failed to delete tenant");
      setIsLoading(false);
      return;
    }

    removeTenant(tenant);

    setIsLoading(false);

    toast.success("Tenant deleted successfully!");

    onOpenChange(false);
  }

  return (
    <Dialog open={isOpened} onOpenChange={(value) => onOpenChange(value)}>
      <DialogContent className="sm:max-w-[450px]">
        <DialogHeader>
          <DialogTitle>Delete Tenant</DialogTitle>

          <DialogDescription>
            On deleting this tenant, it will be permanently removed and
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
