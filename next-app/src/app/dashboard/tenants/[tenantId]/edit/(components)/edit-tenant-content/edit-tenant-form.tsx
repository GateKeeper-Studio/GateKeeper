"use client";

import { z } from "zod";
import { toast } from "sonner";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";

import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";

import { formSchema } from "./schema";

import { editTenantApi } from "@/services/settings/edit-tenant";

type Props = {
  tenant: {
    id: string;
    name: string;
    description: string;
  };
};

export function EditTenantForm({ tenant }: Props) {
  const router = useRouter();
  const [isLoading, setIsLoading] = useState(false);

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      name: tenant.name,
      description: tenant.description,
    },
  });

  async function onSubmit(values: z.infer<typeof formSchema>) {
    setIsLoading(true);

    const [, err] = await editTenantApi(
      { ...values, id: tenant.id },
      {
        accessToken: "fake-token",
      },
    );

    if (err) {
      console.error(err);
      toast.error("An error occurred while updating the tenant.");
      setIsLoading(false);
      return;
    }

    toast.success("Tenant updated successfully!");

    setIsLoading(false);

    router.push("/dashboard/tenants");
  }

  return (
    <Form {...form}>
      <form
        onSubmit={form.handleSubmit(onSubmit)}
        className="mt-4 max-w-[700px]"
      >
        <div className="grid gap-2">
          <FormField
            control={form.control}
            name="name"
            render={({ field }) => (
              <FormItem>
                <FormLabel className="flex gap-1">
                  Name
                  <span className="text-red-500">*</span>
                </FormLabel>

                <FormControl>
                  <Input
                    placeholder="Type the tenant name"
                    autoComplete="name"
                    type="text"
                    {...field}
                  />
                </FormControl>

                <FormDescription></FormDescription>
                <FormMessage></FormMessage>
              </FormItem>
            )}
          />

          <FormField
            control={form.control}
            name="description"
            render={({ field }) => (
              <FormItem>
                <FormLabel className="flex gap-1">Description</FormLabel>

                <FormControl>
                  <Textarea
                    placeholder="Type the tenant description"
                    autoComplete="description"
                    {...field}
                  />
                </FormControl>

                <FormDescription></FormDescription>
                <FormMessage></FormMessage>
              </FormItem>
            )}
          />

          <Button type="submit" disabled={isLoading} className="ml-auto w-fit">
            {isLoading ? "Editing Tenant..." : "Edit Tenant"}
          </Button>
        </div>
      </form>
    </Form>
  );
}
