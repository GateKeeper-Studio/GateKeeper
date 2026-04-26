"use client";

import z from "zod";
import { toast } from "sonner";
import { useState } from "react";
import { useRouter } from "next/navigation";

import { Trash } from "lucide-react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";

import { Separator } from "@/components/ui/separator";
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
import {
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
  Form,
} from "@/components/ui/form";
import { Spinner } from "@/components/ui/spinner";

import { deleteTenantApi } from "@/services/dashboard/delete-tenant";
import type { Tenant } from "@/services/dashboard/use-tenants-swr";

type Props = {
  onOpenChange: (value: boolean) => void;
  isModalOpened: boolean;
  tenant: Tenant | null;
};

export function DeleteTenantDialog({
  isModalOpened,
  onOpenChange,
  tenant,
}: Props) {
  const router = useRouter();
  const [isLoading, setIsLoading] = useState(false);

  async function submit() {
    if (!tenant) {
      toast.error("Tenant data is missing.");
      return;
    }

    setIsLoading(true);

    const [err] = await deleteTenantApi(
      { tenantId: tenant.id },
      { accessToken: "" },
    );

    if (err) {
      console.error(err);
      toast.error(
        err?.response?.data.message || "Failed to delete the tenant.",
      );

      setIsLoading(false);
      return;
    }

    setIsLoading(false);
    onOpenChange(false);

    setTimeout(() => {
      router.push("/dashboard/projects");
    });

    toast.success("Tenant deleted successfully.");
  }

  const validation = z.object({
    confirmation: z.string().refine((value) => value === tenant?.name, {
      message: "Tenant name does not match.",
    }),
  });

  const form = useForm<z.infer<typeof validation>>({
    resolver: zodResolver(validation as any),
    defaultValues: {
      confirmation: "",
    },
  });

  return (
    <Dialog open={isModalOpened} onOpenChange={onOpenChange}>
      <DialogTrigger
        type="button"
        className={buttonVariants({ variant: "destructive" })}
      >
        <Trash /> Delete this Project
      </DialogTrigger>

      <DialogContent className="flex flex-col  overflow-hidden  p-0 gap-0">
        <DialogHeader className="p-6 flex flex-col gap-2">
          <DialogTitle>Delete Tenant</DialogTitle>

          <DialogDescription>
            On delete this tenant, it will affect all the data related to
            this tenant. Are you sure?
          </DialogDescription>

          <DialogDescription className="text-sm text-muted-foreground mt-4">
            Type the tenant name{" "}
            <strong className="font-semibold text-destructive">
              “{tenant?.name}”
            </strong>
            .
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form className="flex flex-col gap-4 px-4 pb-4">
            <FormField
              control={form.control}
              name={`confirmation`}
              render={({ field }) => (
                <FormItem className="w-full">
                  <FormLabel className="flex gap-1">Confirmation</FormLabel>

                  <FormControl>
                    <Input
                      placeholder="Type the tenant name"
                      autoComplete="name"
                      type="text"
                      {...field}
                    />
                  </FormControl>

                  <FormMessage />
                </FormItem>
              )}
            />

            <span className="text-muted-foreground italic px-4 pb-4 text-sm">
              On delete this tenant, it will affect all the data related
              to this tenant, but you can recover it from the trash
              section within <strong className="font-semibold">30 days</strong>.
            </span>
          </form>
        </Form>

        <Separator />

        <DialogFooter className="p-4 gap-4 mt-auto">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>

          <Button
            type="button"
            disabled={isLoading}
            onClick={form.handleSubmit(submit)}
            className="disabled:opacity-50"
            variant="destructive"
          >
            {isLoading ? (
              <>
                <Spinner />
                Deleting...
              </>
            ) : (
              "Confirm"
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
