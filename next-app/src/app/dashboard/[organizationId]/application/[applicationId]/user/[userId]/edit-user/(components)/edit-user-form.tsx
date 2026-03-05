"use client";

import { z } from "zod";
import { toast } from "sonner";
import { useState } from "react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm, useWatch } from "react-hook-form";
import { useParams, useRouter } from "next/navigation";

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
import { Switch } from "@/components/ui/switch";
import { Checkbox } from "@/components/ui/checkbox";
import { Separator } from "@/components/ui/separator";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

import { formSchema } from "../../schema";
import { ResetPasswordDialog } from "../../(components)/reset-password-dialog";
import { MultiFactorAuthForm } from "../../(components)/multi-factor-auth-form";
import { ApplicationRolesSection } from "../../(components)/application-roles-section";

import { UserByIdResponse } from "@/services/dashboard/get-tenant-user-by-id";
import { editTenantUserApi } from "@/services/dashboard/edit-tenant-user";
import { useApplicationsSWR } from "@/services/dashboard/use-applications-swr";

type Props = {
  user: UserByIdResponse | null;
};

export function EditUserForm({ user }: Props) {
  const [isLoading, setIsLoading] = useState(false);
  const [selectedAppId, setSelectedAppId] = useState("");
  const router = useRouter();

  const { organizationId, userId } = useParams() as {
    organizationId: string;
    userId: string;
  };

  const { data: applications } = useApplicationsSWR(
    { organizationId },
    { accessToken: "" },
  );

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      displayName: user?.displayName || "",
      email: user?.email || "",
      firstName: user?.firstName || "",
      lastName: user?.lastName || "",
      roles: user?.badges.map((role) => role.id) || [],
      temporaryPassword: "",
      preferred2FAMethod: user?.preferred2FAMethod || null,
      isEmailConfirmed: user?.isEmailVerified || false,
      isActive: user?.isActive || false,
      IsMfaAuthAppConfigured: user?.isMfaAuthAppConfigured || false,
      isMfaEmailConfigured: user?.isMfaEmailConfigured || false,
      isMfaWebauthnConfigured: user?.isMfaWebauthnConfigured || false,
    },
  });

  async function onSubmit(values: z.infer<typeof formSchema>) {
    setIsLoading(true);

    const [, err] = await editTenantUserApi(
      {
        applicationId: selectedAppId,
        organizationId,
        userId,
        displayName: values.displayName,
        email: values.email,
        firstName: values.firstName,
        lastName: values.lastName,
        isEmailConfirmed: values.isEmailConfirmed,
        roles: values.roles,
        preferred2FAMethod: values.preferred2FAMethod,
        temporaryPasswordHash: values.temporaryPassword || null,
        isActive: values.isActive,
      },
      { accessToken: "" },
    );

    if (err) {
      console.error(err);
      toast.error(err.response?.data.message || err.message);
      setIsLoading(false);
      return;
    }

    setIsLoading(false);
    toast.success("User updated successfully.");

    router.push(`/dashboard/${organizationId}/users/${userId}`);
  }

  const temporaryPassword = useWatch({
    control: form.control,
    name: "temporaryPassword",
    defaultValue: "",
  });

  const isActive = useWatch({
    control: form.control,
    name: "isActive",
    defaultValue: user?.isActive || false,
  });

  return (
    <Form {...form}>
      <div className="mt-4 flex flex-col gap-1">
        <FormField
          control={form.control}
          name="isActive"
          render={({ field }) => (
            <FormItem className="w-full">
              <FormLabel className="flex gap-1">Status</FormLabel>

              <div className="w-full flex gap-2">
                <FormControl>
                  <Switch
                    checked={!!field.value}
                    aria-labelledby="status-label"
                    onCheckedChange={field.onChange}
                  />
                </FormControl>

                <span
                  className="text-muted-foreground font-semibold text-xs data-[isactive=true]:text-green-500 data-[isactive=false]:text-red-500"
                  data-isactive={isActive}
                >
                  {isActive ? "Enabled" : "Disabled"}
                </span>
              </div>
            </FormItem>
          )}
        />
      </div>

      <form
        onSubmit={form.handleSubmit(onSubmit)}
        className="mt-4 max-w-[700px]"
      >
        <div className="grid gap-4">
          <FormField
            control={form.control}
            name="displayName"
            render={({ field }) => (
              <FormItem className="w-full">
                <FormLabel className="flex gap-1">
                  Display Name
                  <span className="text-red-500">*</span>
                </FormLabel>

                <FormControl>
                  <Input
                    placeholder="Type the user display name"
                    autoComplete="name"
                    type="text"
                    {...field}
                  />
                </FormControl>

                <FormDescription>
                  The name that will be displayed to the user.
                </FormDescription>
                <FormMessage />
              </FormItem>
            )}
          />

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

                  <FormControl>
                    <Input
                      placeholder="Type the user first name"
                      autoComplete="given-name"
                      type="text"
                      {...field}
                    />
                  </FormControl>

                  <FormMessage />
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

                  <FormControl>
                    <Input
                      placeholder="Type the user last name"
                      autoComplete="family-name"
                      type="text"
                      {...field}
                    />
                  </FormControl>
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

                <FormControl>
                  <Input
                    placeholder="Type the user e-mail"
                    autoComplete="email"
                    type="text"
                    {...field}
                  />
                </FormControl>
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
                <FormMessage />
              </FormItem>
            )}
          />

          <div className="flex flex-col gap-1">
            <span className="text-sm font-medium">Reset User Password</span>

            <span className="text-muted-foreground my-2 text-sm">
              Reset the user password. On click, the user will receive an e-mail
              with the new password, and the user will be required to change it
              on the next login.
            </span>

            <ResetPasswordDialog form={form} />

            {temporaryPassword && (
              <span className="text-orange-500 font-semibold text-sm">
                The user password was changed! The user will be required to set
                a new password on the next login.
              </span>
            )}
          </div>

          <Separator className="my-2" />

          <MultiFactorAuthForm
            isEditEnabled={true}
            form={form}
            userId={userId}
            applicationId={selectedAppId}
          />

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

          <ApplicationRolesSection
            isEditEnabled={true}
            form={form}
            applicationId={selectedAppId}
          />
        </div>

        <Button type="submit" className="float-right mt-4" disabled={isLoading}>
          {isLoading && <LoadingSpinner />}
          Apply Changes
        </Button>
      </form>
    </Form>
  );
}
