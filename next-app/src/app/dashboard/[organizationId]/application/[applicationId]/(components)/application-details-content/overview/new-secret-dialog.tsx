"use client";

import { toast } from "sonner";
import { useState } from "react";
import { CircleAlert } from "lucide-react";
import { useParams } from "next/navigation";

import { createApplicationSecretApi } from "@/services/dashboard/create-application-secret";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { DatePicker } from "@/components/ui/date-picker";
import { Button, buttonVariants } from "@/components/ui/button";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

import { formatDate } from "@/lib/utils";
import { IApplication } from "@/services/dashboard/get-application-by-id";

type Props = {
  addSecret(newSecret: IApplication["secrets"][number]): void;
};
export function NewSecretDialog({ addSecret }: Props) {
  const params = useParams();

  const applicationId = params.applicationId as string;
  const organizationId = params.organizationId as string;

  const [isLoading, setIsLoading] = useState(false);
  const [secret, setSecret] = useState("");
  const [copied, setCopied] = useState(false);

  const [secretName, setSecretName] = useState("");
  const [expiresAt, setExpiresAt] = useState<Date | null>(null);

  async function generate() {
    setIsLoading(true);

    const [response, err] = await createApplicationSecretApi(
      {
        name: secretName,
        expiresAt: expiresAt || null,
        applicationId,
        organizationId,
      },
      { accessToken: "" }
    );

    if (err) {
      toast.error("Failed to generate secret");
      console.error(err);
      return;
    }

    setSecret(response?.value || "");
    addSecret(response as unknown as IApplication["secrets"][number]);

    setIsLoading(false);
  }

  function copySecret(secret: string) {
    navigator.clipboard.writeText(secret).then(() => {
      setCopied(true);

      setTimeout(() => setCopied(false), 1000);
    });
  }

  function clear() {
    setSecret("");
    setSecretName("");
    setExpiresAt(null);
  }

  return (
    <Dialog onOpenChange={(isOpened) => isOpened && clear()}>
      <DialogTrigger className={buttonVariants({ variant: "default" })}>
        New Secret
      </DialogTrigger>

      <DialogContent className="top-[40%] sm:max-w-[520px]">
        <DialogHeader>
          <DialogTitle>New Application Secret</DialogTitle>

          <DialogDescription>
            Generate a new secret for your application. Keep it safe.
          </DialogDescription>
        </DialogHeader>

        {secret ? (
          <>
            <Alert className="bg-orange-500/10">
              <CircleAlert />

              <AlertTitle>Wait!</AlertTitle>
              <AlertDescription>
                The secret will be only visible at this moment. Save it on
                another safe place or copy to use now.
              </AlertDescription>
            </Alert>

            <div className="relative flex items-center justify-between gap-4 rounded-md bg-gray-100 dark:bg-gray-900 p-4">
              <span className="w-full text-center text-lg font-bold">
                {secret}
              </span>

              <Button
                onClick={copySecret.bind(null, secret)}
                className="min-w-[75px]"
              >
                {copied ? "Copied!" : "Copy"}
              </Button>
            </div>

            <div className="relative flex items-center text-lg justify-center gap-4 rounded-md bg-gray-100 dark:bg-gray-900 p-4">
              Expire at:
              <span className="text-primary font-medium">
                {expiresAt ? formatDate(expiresAt) : "Lifetime"}
              </span>
            </div>
          </>
        ) : (
          <>
            <div className="grid gap-4 py-4">
              <div className="flex flex-col gap-3">
                <Label htmlFor="name">Secret Name</Label>
                <Input
                  id="name"
                  placeholder="E.g: My Ultra Application Secret"
                  className="col-span-3"
                  value={secretName}
                  onChange={(e) => setSecretName(e.target.value)}
                />
              </div>

              <div className="flex flex-col gap-3">
                <Label htmlFor="username">Expires At</Label>

                <div className="flex items-end gap-2">
                  <DatePicker
                    value={expiresAt}
                    onSelect={(date) => setExpiresAt(date)}
                  />

                  <Button
                    variant="outline"
                    onClick={() => setExpiresAt(null)}
                    className="text-sm"
                  >
                    Clear
                  </Button>
                </div>
              </div>
            </div>

            <DialogFooter>
              <Button type="submit" onClick={generate}>
                {isLoading ? "Generating..." : "Generate"}
              </Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
