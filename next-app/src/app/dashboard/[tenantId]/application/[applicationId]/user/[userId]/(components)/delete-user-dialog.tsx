"use client";

import { toast } from "sonner";
import { useState } from "react";
import { Trash } from "lucide-react";
import { useParams, useRouter } from "next/navigation";

import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button, buttonVariants } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { LoadingSpinner } from "@/components/ui/loading-spinner";

import { deleteTenantUserApi } from "@/services/dashboard/delete-tenant-user";

export function DeleteUserDialog() {
  const [isLoading, setIsLoading] = useState(false);
  const { userId, tenantId } = useParams() as {
    userId: string;
    tenantId: string;
  };

  const router = useRouter();

  async function handler() {
    setIsLoading(true);

    const [err] = await deleteTenantUserApi(
      { tenantId, userId },
      { accessToken: "" },
    );

    if (err) {
      setIsLoading(false);
      console.error(err);
      toast.error(err.response?.data.message || "Something went wrong");
      return;
    }

    setIsLoading(false);

    toast.success("User deleted successfully");

    router.push(`/dashboard/${tenantId}/users`);
  }

  return (
    <Dialog>
      <Tooltip delayDuration={0}>
        <TooltipTrigger asChild>
          <DialogTrigger className={buttonVariants({ variant: "destructive" })}>
            <Trash />
          </DialogTrigger>
        </TooltipTrigger>

        <TooltipContent>Delete User</TooltipContent>
      </Tooltip>

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

          <Button
            type="submit"
            onClick={handler}
            variant="destructive"
            disabled={isLoading}
          >
            {isLoading && <LoadingSpinner className="absolute left-4" />}
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
