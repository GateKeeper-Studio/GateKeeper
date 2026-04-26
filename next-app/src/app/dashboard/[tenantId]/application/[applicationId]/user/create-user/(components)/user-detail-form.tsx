"use client";

import { z } from "zod";
import { toast } from "sonner";
import { useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm, UseFormReturn } from "react-hook-form";

import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Separator } from "@/components/ui/separator";

import { formSchema } from "../schema";
import { MultiFactorAuthForm } from "./multi-factor-auth-form";
import { ApplicationRolesSection } from "./application-roles-section";
import { createTenantUserApi } from "@/services/dashboard/create-tenant-user";
import { useApplicationsSWR } from "@/services/dashboard/use-applications-swr";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

export type FormType = UseFormReturn<z.infer<typeof formSchema>>;

export function UserDetailForm() {
  const { tenantId } = useParams() as {
    tenantId: string;
  };
  const [isLoading, setIsLoading] = useState(false);
  const [selectedAppId, setSelectedAppId] = useState("");

  const router = useRouter();

  const { data: applications } = useApplicationsSWR(
    { tenantId },
    { accessToken: "" },
  );

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      displayName: "",
      email: "",
      firstName: "",
      isEmailConfirmed: false,
      lastName: "",
      hasMfaAuthAppEnabled: false,
      hasMfaEmailEnabled: false,
      roles: [],
    },
  });

  async function onSubmit(values: z.infer<typeof formSchema>) {
    setIsLoading(true);

    const [response, err] = await createTenantUserApi(
      {
        applicationId: selectedAppId,
        tenantId,
        displayName: values.displayName,
        email: values.email,
        firstName: values.firstName,
        lastName: values.lastName,
        isEmailConfirmed: values.isEmailConfirmed,
        isMfaAuthAppEnabled: values.hasMfaAuthAppEnabled,
        isMfaEmailEnabled: values.hasMfaEmailEnabled,
        roles: values.roles,
        temporaryPasswordHash: values.temporaryPassword,
      },
      { accessToken: "" },
    );

    if (err) {
      console.error(err);
      toast.error(
        err.response?.data.message ||
          "An error occurred while creating the user.",
      );
      setIsLoading(false);
      return;
    }

    setIsLoading(false);

    router.push(`/dashboard/${tenantId}/users/${response?.id}`);
  }

  return (
    <Form {...form}>
      <div className="flex items-center justify-between gap-4">
        <FormField
          control={form.control}
          name="displayName"
          render={({ field }) => (
            <FormItem className="w-full">
              <FormLabel className="flex gap-1 sr-only">
                Display Name
                <span className="text-red-500">*</span>
              </FormLabel>

              <div className="w-full flex gap-2">
                <FormControl>
                  <Input
                    placeholder="Type the user display name"
                    autoComplete="name"
                    type="text"
                    style={{
                      fontSize: "1.75rem",
                      fontWeight: 700,
                      height: "3.5rem",
                      lineHeight: "3.5rem",
                    }}
                    className="max-w-[700px]"
                    {...field}
                  />
                </FormControl>
              </div>

              <FormDescription>
                The name that will be displayed to the user.
              </FormDescription>
              <FormMessage></FormMessage>
            </FormItem>
          )}
        />
      </div>

      <form
        onSubmit={form.handleSubmit(onSubmit)}
        className="mt-4 max-w-[700px]"
      >
        <div className="grid gap-4">
          <fieldset className="flex gap-2">
            <FormField
              control={form.control}
              name="firstName"
              render={({ field }) => (
                <FormItem className="w-full">
                  <FormLabel className="flex gap-1">
                    First Name
                    <span className="text-red-500">*</span>
                  </FormLabel>

                  <div className="w-full flex gap-2">
                    <FormControl>
                      <Input
                        className="w-full"
                        placeholder="Type the user first name"
                        autoComplete="given-name"
                        type="text"
                        {...field}
                      />
                    </FormControl>
                  </div>

                  <FormMessage></FormMessage>
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="lastName"
              render={({ field }) => (
                <FormItem className="w-full">
                  <FormLabel className="flex gap-1">
                    Last Name
                    <span className="text-red-500">*</span>
                  </FormLabel>

                  <div className="w-full flex gap-2">
                    <FormControl>
                      <Input
                        className="w-full"
                        placeholder="Type the user last name"
                        autoComplete="family-name"
                        type="text"
                        {...field}
                      />
                    </FormControl>
                  </div>

                  <FormMessage></FormMessage>
                </FormItem>
              )}
            />
          </fieldset>

          <Separator className="my-2" />

          <FormField
            control={form.control}
            name="email"
            render={({ field }) => (
              <FormItem>
                <FormLabel className="flex gap-1">
                  E-mail
                  <span className="text-red-500">*</span>
                </FormLabel>

                <div className="w-full flex gap-2">
                  <FormControl>
                    <Input
                      className="w-full"
                      placeholder="Type the user e-mail"
                      autoComplete="email"
                      type="text"
                      {...field}
                    />
                  </FormControl>
                </div>
                <FormMessage></FormMessage>
              </FormItem>
            )}
          />

          <FormField
            control={form.control}
            name="isEmailConfirmed"
            render={({ field }) => (
              <FormItem className="w-full p-3 rounded-lg bg-gray-50 dark:bg-gray-900 shadow">
                <div className="flex items-center space-x-2">
                  <FormControl>
                    <Checkbox
                      checked={!!field.value}
                      onCheckedChange={field.onChange}
                      aria-labelledby="terms-label"
                      id="is-email-confirmed"
                      className="bg-background"
                    />
                  </FormControl>

                  <FormLabel
                    htmlFor="is-email-confirmed"
                    className="text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                  >
                    Is e-mail already confirmed?
                  </FormLabel>
                </div>

                <FormDescription>
                  If the user e-mail is already confirmed, check this box.
                </FormDescription>
                <FormMessage></FormMessage>
              </FormItem>
            )}
          />

          <FormField
            control={form.control}
            name="temporaryPassword"
            render={({ field }) => (
              <FormItem className="w-full">
                <FormLabel className="flex gap-1">
                  Temporary Password
                  <span className="text-red-500">*</span>
                </FormLabel>

                <div className="w-full flex gap-2">
                  <FormControl>
                    <Input
                      className="w-full"
                      placeholder="Type the user temporary password"
                      autoComplete="password"
                      type="password"
                      {...field}
                    />
                  </FormControl>
                </div>

                <FormDescription>
                  The temporary password that will be sent to the user at the
                  first access.
                </FormDescription>
                <FormMessage></FormMessage>
              </FormItem>
            )}
          />

          <Separator className="my-2" />

          <MultiFactorAuthForm form={form} />

          <Separator className="my-2" />

          <div className="flex flex-col gap-1">
            <span className="text-sm font-medium">Application Context</span>
            <span className="text-muted-foreground my-1 text-sm">
              Select the application to assign roles from.
            </span>
            <Select onValueChange={setSelectedAppId} value={selectedAppId}>
              <SelectTrigger className="max-w-xs">
                <SelectValue placeholder="Select an application" />
              </SelectTrigger>
              <SelectContent>
                {applications?.map((app) => (
                  <SelectItem key={app.id} value={app.id}>
                    {app.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <ApplicationRolesSection form={form} applicationId={selectedAppId} />
        </div>

        <Button type="submit" className="float-right mt-4" disabled={isLoading}>
          Create User
        </Button>
      </form>
    </Form>
  );
}
