import { toast } from "sonner";
import { useState } from "react";
import { useParams } from "next/navigation";

import { Button, buttonVariants } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";

import { ApplicationRoleItem } from "@/services/dashboard/use-application-roles-swr";
import { createApplicationRoleApi } from "@/services/dashboard/create-application-role";

type Props = {
  addRole: (role: ApplicationRoleItem) => void;
};

export function NewRoleDialog({ addRole }: Props) {
  const [isOpened, setIsOpened] = useState(false);

  const { tenantId, applicationId } = useParams() as {
    tenantId: string;
    applicationId: string;
  };

  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  async function create() {
    setIsLoading(true);

    const [response, err] = await createApplicationRoleApi(
      { tenantId, applicationId, name, description },
      { accessToken: "" },
    );

    if (err) {
      console.error(err);
      toast.error("An error occurred. Please try again later.");
      setIsLoading(false);
      return;
    }

    if (response) {
      addRole(response);
    }

    setIsOpened(false);
    setIsLoading(false);

    clear();
  }

  function clear() {
    setName("");
    setDescription("");
  }

  return (
    <Dialog
      open={isOpened}
      onOpenChange={(isOpenedState) => {
        setIsOpened(isOpenedState);

        if (!isOpenedState) {
          clear();
        }
      }}
    >
      <DialogTrigger className={buttonVariants({ variant: "default" })}>
        Add Role
      </DialogTrigger>

      <DialogContent className="sm:max-w-[450px]">
        <DialogHeader>
          <DialogTitle>New Application Role</DialogTitle>
          <DialogDescription>
            Create a new role for your application. Handle permissions and
            access.
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 py-4">
          <div className="flex flex-col gap-3">
            <Label htmlFor="name">
              Name <span className="text-red-500">*</span>
            </Label>
            <Input
              id="name"
              placeholder="Type the role name"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>

          <div className="flex flex-col gap-3">
            <Label htmlFor="description">
              Description <span className="text-red-500">*</span> (
              {120 - description.length})
            </Label>

            <Textarea
              id="description"
              placeholder="Type the role description"
              value={description}
              maxLength={120}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>
        </div>

        <DialogFooter>
          <Button type="submit" onClick={create}>
            {isLoading ? "Creating..." : "Create"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
