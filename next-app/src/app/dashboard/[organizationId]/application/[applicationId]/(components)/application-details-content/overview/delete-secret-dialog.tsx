"use client";

import { toast } from "sonner";
import { useState } from "react";
import { Trash } from "lucide-react";
import { useParams } from "next/navigation";

import { Button, buttonVariants } from "@/components/ui/button";
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

import { deleteApplicationSecretApi } from "@/services/dashboard/delete-application-secret";
import { IApplication } from "@/services/dashboard/get-application-by-id";

type Props = {
  secret: IApplication["secrets"][number];
  removeSecret: () => void;
};

export function DeleteSecretDialog({ secret, removeSecret }: Props) {
  const [isLoading, setIsLoading] = useState(false);
  const params = useParams();

  const { applicationId, organizationId } = params as {
    applicationId: string;
    organizationId: string;
  };

  async function handler() {
    if (!applicationId) {
      toast.error("Application ID is missing");
      return;
    }

    setIsLoading(true);

    const [err] = await deleteApplicationSecretApi(
      {
        applicationId: applicationId,
        secretId: secret.id,
        organizationId,
      },
      { accessToken: "" }
    );

    if (err) {
      toast.error("Failed to delete secret");
      console.error(err);
      setIsLoading(false);
      return;
    }

    removeSecret();

    // Logic here
    setIsLoading(false);
  }

  return (
    <Dialog>
      <DialogTrigger className={buttonVariants({ variant: "outline" })}>
        <Trash />
      </DialogTrigger>

      <DialogContent className="sm:max-w-[450px]">
        <DialogHeader>
          <DialogTitle>Delete Application Secret</DialogTitle>
          <DialogDescription>
            On deleting this secret ({secret.name}), it will be permanently
            removed from the server. Are you sure?
          </DialogDescription>
        </DialogHeader>

        <DialogFooter>
          <DialogClose className={buttonVariants({ variant: "outline" })}>
            Cancel
          </DialogClose>

          <Button type="submit" onClick={handler}>
            {isLoading ? "Deleting..." : "Delete"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
