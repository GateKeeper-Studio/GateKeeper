import { useState } from "react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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

import { cn } from "@/lib/utils";
import { FormType } from "../schema";

type Props = {
  form: FormType;
};

export function ResetPasswordDialog({ form }: Props) {
  const [draftTemporaryPassword, setDraftTemporaryPassword] = useState("");
  const [isDialogOpen, setIsDialogOpen] = useState(false);

  function handler() {
    form.setValue("temporaryPassword", draftTemporaryPassword);
    setIsDialogOpen(false);
  }

  function clear() {
    form.setValue("temporaryPassword", "");
    setDraftTemporaryPassword("");
    setIsDialogOpen(false);
  }

  return (
    <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
      <DialogTrigger
        className={cn(buttonVariants({ variant: "secondary" }), "w-fit")}
      >
        Reset Password
      </DialogTrigger>

      <DialogContent className="sm:max-w-[550px]">
        <DialogHeader>
          <DialogTitle>Reset Password</DialogTitle>
          <DialogDescription>
            On confirm, the user will receive an e-mail with the new password,
            and the user will be required to change it on the next login.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-3">
          <Label htmlFor="temp-password-input">Temporary Password</Label>
          <div className="flex gap-4">
            <Input
              id="temp-password-input"
              type="password"
              placeholder="Type the temporary password"
              value={draftTemporaryPassword}
              onChange={(e) => setDraftTemporaryPassword(e.target.value)}
            />

            <Button type="button" onClick={clear}>
              Clear
            </Button>
          </div>
        </div>

        <DialogFooter>
          <DialogClose className={buttonVariants({ variant: "outline" })}>
            Cancel
          </DialogClose>

          <Button type="button" onClick={handler}>
            Confirm
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
