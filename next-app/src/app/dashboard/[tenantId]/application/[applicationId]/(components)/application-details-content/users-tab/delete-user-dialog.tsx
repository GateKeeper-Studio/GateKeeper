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
import { Input } from "@/components/ui/input";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";

import type { UserTableItem } from "./users-table";

type Props = {
  isOpened: boolean;
  onOpenChange: (value: boolean) => void;
  users: UserTableItem[];
  removeTableSelection: () => void;
  removeUsers: (users: UserTableItem[]) => void;
};

export function DeleteUserDialog({
  isOpened,
  onOpenChange,
  users,
  removeTableSelection,
  removeUsers,
}: Props) {
  const [confirmationValue, setConfirmationValue] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  async function handler() {
    if (!users || users.length === 0) {
      console.error("No users are defined");
      toast.error("No users are defined");
      return;
    }

    if (confirmationValue !== "Yes, I want to delete the selected users") {
      toast.error("Confirmation value does not match");
      return;
    }

    setIsLoading(true);

    removeUsers(users);
    removeTableSelection();

    setIsLoading(false);
    onOpenChange(false);

    toast.success("Selected users deleted successfully");
  }

  return (
    <Dialog open={isOpened} onOpenChange={(value) => onOpenChange(value)}>
      <DialogContent className="sm:max-w-[450px]">
        <DialogHeader>
          <DialogTitle>Delete User</DialogTitle>

          <DialogDescription>
            On deleting the selected users, it will affect all the data related
            to this application. Are you sure?
          </DialogDescription>

          <DialogDescription>
            The users are:{" "}
            <span className="text-primary font-semibold">
              {users.map((user) => user.displayName).join(", ")}
            </span>
          </DialogDescription>

          <DialogDescription>
            Type{" "}
            <span className="text-destructive font-semibold">
              Yes, I want to delete the selected users
            </span>{" "}
            to confirm
          </DialogDescription>
        </DialogHeader>

        <Field>
          <FieldLabel htmlFor="input-demo-api-key">Confirmation</FieldLabel>
          <Input
            id="input-demo-api-key"
            type="text"
            placeholder="Yes, I want to delete the selected users"
            value={confirmationValue}
            onChange={(e) => setConfirmationValue(e.target.value)}
          />
          <FieldDescription className="italic text-orange-400">
            The selected users will not be deleted, only disabled. You will be
            able to enable them again.
          </FieldDescription>
        </Field>

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
