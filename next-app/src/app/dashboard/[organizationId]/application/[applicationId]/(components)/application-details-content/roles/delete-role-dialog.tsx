"use client";

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
import { ApplicationRole } from "./roles-table";
import { useParams } from "next/navigation";
import { deleteApplicationRoleApi } from "@/services/dashboard/delete-application-role";
import { toast } from "sonner";

type Props = {
  isOpened: boolean;
  onOpenChange: (isOpened: boolean) => void;
  role: ApplicationRole | null;
  removeRole: (role: ApplicationRole) => void;
};

export function DeleteRoleDialog({
  role,
  isOpened,
  onOpenChange,
  removeRole,
}: Props) {
  const { organizationId, applicationId } = useParams() as {
    organizationId: string;
    applicationId: string;
  };

  const [isLoading, setIsLoading] = useState(false);

  async function handler() {
    if (!role) {
      console.error("Role is not defined");
      toast.error("Role is not defined");
      return;
    }

    setIsLoading(true);

    const [err] = await deleteApplicationRoleApi(
      { applicationId, organizationId, roleId: role?.id },
      { accessToken: "" },
    );

    if (err) {
      console.error(err);
      toast.error("Failed to delete role");
      setIsLoading(false);
      return;
    }

    removeRole(role);

    // Logic here
    setIsLoading(false);

    onOpenChange(false);
  }

  return (
    <Dialog open={isOpened} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[450px]">
        <DialogHeader>
          <DialogTitle>Delete User</DialogTitle>

          <DialogDescription>
            On deleting this user, it will be permanently removed from the
            application. Are you sure?
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
