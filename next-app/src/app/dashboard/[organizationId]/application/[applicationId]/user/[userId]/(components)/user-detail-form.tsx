"use client";

import { z } from "zod";
import { toast } from "sonner";
import { useState } from "react";
import { Copy, Pencil } from "lucide-react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm, UseFormReturn, useWatch } from "react-hook-form";
import { useParams, useRouter, useSearchParams } from "next/navigation";

import { Badge } from "@/components/ui/badge";
import { Button, buttonVariants } from "@/components/ui/button";
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
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { cn, copy } from "@/lib/utils";

import { formSchema } from "../schema";
import { DeleteUserDialog } from "./delete-user-dialog";
import { ResetPasswordDialog } from "./reset-password-dialog";
import { MultiFactorAuthForm } from "./multi-factor-auth-form";
import { ApplicationRolesSection } from "./application-roles-section";

import { UserByIdResponse } from "@/services/dashboard/get-application-user-by-id";
import { editApplicationUserApi } from "@/services/dashboard/edit-application-user";

type Props = {
  user: UserByIdResponse | null;
};

export type FormType = UseFormReturn<z.infer<typeof formSchema>>;

export function UserDetailForm({ user }: Props) {
  const searchParams = useSearchParams();
  const isEditable = searchParams.get("edit") === "true";

  const [isLoading, setIsLoading] = useState(false);
  const [isEditEnabled, setIsEditEnabled] = useState(isEditable);

  const router = useRouter();

  const { applicationId, organizationId, userId } = useParams() as {
    organizationId: string;
    applicationId: string;
    userId: string;
  };

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

    const [, err] = await editApplicationUserApi(
      {
        applicationId: applicationId,
        organizationId: organizationId,
        userId: userId,
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

    router.push(
      `/dashboard/${organizationId}/application/${applicationId}/user/${userId}`,
    );

    setIsEditEnabled(false);
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

  function reset() {
    form.setValue("temporaryPassword", "");
    form.setValue("isEmailConfirmed", user?.isEmailVerified || false);
    form.setValue("isActive", user?.isActive || false);
    form.setValue("roles", user?.badges.map((role) => role.id) || []);
    form.setValue("preferred2FAMethod", user?.preferred2FAMethod || null);
    form.setValue("displayName", user?.displayName || "");
    form.setValue("email", user?.email || "");
    form.setValue("firstName", user?.firstName || "");
    form.setValue("lastName", user?.lastName || "");
  }

  return (
    <>
      {isEditEnabled && (
        <Badge className="mb-4 w-fit text-sm" title="Edit is enabled">
          Editing
        </Badge>
      )}

      <Form {...form}>
        <div className="flex items-center justify-between gap-4">
          {isEditEnabled ? (
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
          ) : (
            <div className="flex gap-4">
              <h2 className="text-3xl font-bold tracking-tight">
                {user?.displayName}
              </h2>

              <Tooltip delayDuration={0}>
                <TooltipTrigger
                  className={buttonVariants({ variant: "outline" })}
                  onClick={() => copy(form.getValues("displayName"))}
                >
                  <Copy />
                </TooltipTrigger>

                <TooltipContent>Copy display name</TooltipContent>
              </Tooltip>
            </div>
          )}

          <div className="flex gap-1">
            <DeleteUserDialog />

            <Tooltip delayDuration={0}>
              <TooltipTrigger
                className={cn(
                  buttonVariants({ variant: "outline" }),
                  "mb-[6px]",
                )}
                onClick={() => {
                  router.push(
                    `/dashboard/${organizationId}/application/${applicationId}/user/${userId}?edit=${!isEditEnabled}`,
                    { scroll: false },
                  );

                  if (isEditEnabled) {
                    reset();
                  }

                  setIsEditEnabled((state) => !state);
                }}
              >
                <Pencil />
              </TooltipTrigger>

              <TooltipContent>Enable Changes</TooltipContent>
            </Tooltip>
          </div>
        </div>

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
                      disabled={!isEditEnabled}
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
                          data-iseditenabled={isEditEnabled}
                          readOnly={!isEditEnabled}
                          className="data-[iseditenabled=true]:outline-none w-full"
                          placeholder="Type the user first name"
                          autoComplete="given-name"
                          type="text"
                          {...field}
                        />
                      </FormControl>

                      {!isEditEnabled && (
                        <Tooltip delayDuration={0}>
                          <TooltipTrigger
                            className={buttonVariants({ variant: "outline" })}
                            onClick={() => copy(field.value)}
                          >
                            <Copy />
                          </TooltipTrigger>

                          <TooltipContent>Copy first name</TooltipContent>
                        </Tooltip>
                      )}
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
                          data-iseditenabled={isEditEnabled}
                          readOnly={!isEditEnabled}
                          className="data-[iseditenabled=true]:outline-none w-full"
                          placeholder="Type the user last name"
                          autoComplete="family-name"
                          type="text"
                          {...field}
                        />
                      </FormControl>

                      {!isEditEnabled && (
                        <Tooltip delayDuration={0}>
                          <TooltipTrigger
                            className={buttonVariants({ variant: "outline" })}
                            onClick={() => copy(field.value || "")}
                          >
                            <Copy />
                          </TooltipTrigger>

                          <TooltipContent>Copy last name</TooltipContent>
                        </Tooltip>
                      )}
                    </div>
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
                        data-iseditenabled={isEditEnabled}
                        readOnly={!isEditEnabled}
                        className="data-[iseditenabled=true]:outline-none w-full"
                        placeholder="Type the user e-mail"
                        autoComplete="email"
                        type="text"
                        {...field}
                      />
                    </FormControl>

                    {!isEditEnabled && (
                      <Tooltip delayDuration={0}>
                        <TooltipTrigger
                          className={buttonVariants({ variant: "outline" })}
                          onClick={() => copy(field.value || "")}
                        >
                          <Copy />
                        </TooltipTrigger>

                        <TooltipContent>Copy e-mail</TooltipContent>
                      </Tooltip>
                    )}
                  </div>
                </FormItem>
              )}
            />

            {isEditEnabled ? (
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
            ) : (
              <div className="w-full p-3 rounded-lg bg-gray-50 items-center dark:bg-gray-900 shadow flex gap-2">
                <span className="text-primary-background text-sm">
                  Is e-mail already confirmed?
                </span>

                {user?.isEmailVerified ? (
                  <span className="text-green-500 font-semibold">Yes</span>
                ) : (
                  <span className="text-red-500 font-semibold">No</span>
                )}
              </div>
            )}

            {isEditEnabled && (
              <div className="flex flex-col gap-1">
                <span className="text-sm font-medium">Reset User Password</span>

                <span className="text-muted-foreground my-2 text-sm">
                  Reset the user password. On click, the user will receive an
                  e-mail with the new password, and the user will be required to
                  change it on the next login.
                </span>

                <ResetPasswordDialog form={form} />

                {temporaryPassword && (
                  <span className="text-orange-500 font-semibold text-sm">
                    The user password was changed! The user will be required to
                    set a new password on the next login.
                  </span>
                )}
              </div>
            )}

            <Separator className="my-2" />

            <MultiFactorAuthForm
              isEditEnabled={isEditEnabled}
              form={form}
              userId={userId}
              applicationId={applicationId}
            />

            <Separator className="my-2" />

            <ApplicationRolesSection
              isEditEnabled={isEditEnabled}
              form={form}
            />
          </div>

          {isEditEnabled && (
            <Button
              type="submit"
              className="float-right mt-4"
              disabled={isLoading}
            >
              {isLoading && <LoadingSpinner />}
              Apply Changes
            </Button>
          )}
        </form>
      </Form>
    </>
  );
}
