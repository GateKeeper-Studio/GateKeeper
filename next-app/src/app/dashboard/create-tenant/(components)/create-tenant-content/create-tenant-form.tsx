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
import { Separator } from "@/components/ui/separator";
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
import { StrongPasswordDialog } from "./strong-password-dialog";

import { addTenantApi } from "@/services/settings/add-tenant";

export function CreateTenantForm() {
  const router = useRouter();
  const [isLoading, setIsLoading] = useState(false);
  const [isStrongPasswordModalOpened, setIsStrongPasswordModalOpened] =
    useState(false);

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      name: "",
      description: "",
      passwordHashSecret: "",
    },
  });

  async function onSubmit(values: z.infer<typeof formSchema>) {
    setIsLoading(true);

    const [, err] = await addTenantApi(
      { ...values },
      {
        accessToken: "fake-token",
      },
    );

    if (err) {
      console.error(err);
      toast.error("An error occurred while creating the tenant.");
      setIsLoading(false);
      return;
    }

    toast.success("Tenant created successfully!");

    setIsLoading(false);

    router.push("/dashboard");
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

          <Separator className="my-2" />

          <FormField
            control={form.control}
            name="passwordHashSecret"
            render={({ field }) => (
              <FormItem>
                <FormLabel className="flex gap-1">
                  Password Hash Secret
                  <span className="text-red-500">*</span>
                </FormLabel>
                <FormControl>
                  <Input
                    placeholder="Type the tenant password hash secret"
                    autoComplete="off"
                    type="password"
                    {...field}
                  />
                </FormControl>

                <StrongPasswordDialog
                  setPassword={(value) =>
                    form.setValue("passwordHashSecret", value)
                  }
                  isModalOpened={isStrongPasswordModalOpened}
                  onOpenChange={setIsStrongPasswordModalOpened}
                />

                <FormDescription>
                  This is the secret that will be used to hash all the passwords
                  from users that belong to this tenant.
                </FormDescription>
                <FormMessage></FormMessage>
              </FormItem>
            )}
          />

          <Button type="submit" disabled={isLoading} className="ml-auto w-fit">
            {isLoading ? "Creating Tenant..." : "Create Tenant"}
          </Button>
        </div>
      </form>
    </Form>
  );
}
