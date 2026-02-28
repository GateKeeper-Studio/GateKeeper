"use client";

import { z } from "zod";
import { toast } from "sonner";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useParams, useRouter } from "next/navigation";

import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Textarea } from "@/components/ui/textarea";
import { Separator } from "@/components/ui/separator";
import { MultiSelectInput } from "@/components/ui/multi-select-input";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";

import { formSchema } from "../schema";

import { StrongPasswordDialog } from "./strong-password-dialog";
import { createApplicationApi } from "@/services/dashboard/create-application";
import { useApplicationsSWR } from "@/services/dashboard/use-applications-swr";

export function CreateApplicationForm() {
  const router = useRouter();
  const { organizationId } = useParams() as { organizationId: string };

  const [isLoading, setIsLoading] = useState(false);
  const [isStrongPasswordModalOpened, setIsStrongPasswordModalOpened] =
    useState(false);

  const { mutate } = useApplicationsSWR(
    { organizationId },
    { accessToken: "" },
  );

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      name: "",
      passwordHashSecret: "",
      description: "",
      badges: [],
      hasMfaAuthApp: false,
      hasMfaEmail: false,
      canSelfForgotPass: false,
      canSelfSignUp: false,
    },
  });

  async function onSubmit(values: z.infer<typeof formSchema>) {
    setIsLoading(true);

    const [response, err] = await createApplicationApi(
      { ...values, organizationId },
      {
        accessToken: "fake-access",
      },
    );

    if (err) {
      console.error(err);
      toast.error("An error occurred while creating the application.");
      setIsLoading(false);
      return;
    }

    mutate((state) => {
      if (state && response) {
        return [
          ...state,
          {
            id: response.id,
            name: response.name,
            description: response.description || "",
            badges: response.badges,
            hasMfaAuthApp: response.hasMfaAuthApp,
            hasMfaEmail: response.hasMfaEmail,
            hasMfaWebauthn: response.hasMfaWebauthn,
            createdAt: new Date(),
            updatedAt: null,
            isActive: true,
          },
        ];
      }

      return undefined;
    });

    router.push(`/dashboard/${organizationId}/application/${response?.id}`);

    setIsLoading(false);
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
                    placeholder="Type the application name"
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
                    placeholder="Type the application description"
                    autoComplete="description"
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
            name="badges"
            render={({ field }) => (
              <FormItem>
                <FormLabel>Badges</FormLabel>
                <FormControl>
                  <MultiSelectInput
                    items={field.value}
                    onChange={(values: string[]) =>
                      form.setValue("badges", values)
                    }
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
                    placeholder="Type the application password hash"
                    autoComplete="name"
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
                  from users that are registered to this application.
                </FormDescription>
                <FormMessage></FormMessage>
              </FormItem>
            )}
          />

          <Separator className="my-2" />

          <div className="flex flex-col gap-3">
            <span className="text-sm font-medium">
              Multi Factor Authentication
            </span>

            <span className="text-muted-foreground text-sm">
              Choose the methods that will be used for multi factor
              authentication.
            </span>

            <FormField
              control={form.control}
              name="hasMfaEmail"
              render={({ field }) => (
                <FormItem className="flex items-center space-x-2">
                  <FormControl>
                    <Checkbox
                      checked={!!field.value}
                      onCheckedChange={field.onChange}
                      aria-labelledby="terms-label"
                    />
                  </FormControl>

                  <FormLabel className="text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                    E-mail
                  </FormLabel>

                  <FormDescription></FormDescription>
                  <FormMessage></FormMessage>
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="hasMfaAuthApp"
              render={({ field }) => (
                <FormItem className="flex items-center space-x-2">
                  <FormControl>
                    <Checkbox
                      checked={!!field.value}
                      onCheckedChange={field.onChange}
                      aria-labelledby="terms-label"
                    />
                  </FormControl>

                  <FormLabel className="text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                    Authenticator App (Microsoft, Google, etc)
                  </FormLabel>

                  <FormDescription></FormDescription>
                  <FormMessage></FormMessage>
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="hasMfaWebauthn"
              render={({ field }) => (
                <FormItem className="flex items-center space-x-2">
                  <FormControl>
                    <Checkbox
                      checked={!!field.value}
                      onCheckedChange={field.onChange}
                      aria-labelledby="terms-label"
                    />
                  </FormControl>

                  <FormLabel className="text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                    Passkey (WebAuthn)
                  </FormLabel>

                  <FormDescription></FormDescription>
                  <FormMessage></FormMessage>
                </FormItem>
              )}
            />
          </div>

          <Separator className="my-2" />

          <div className="flex flex-col gap-3">
            <span className="text-sm font-medium">Authentication Page</span>

            <span className="text-muted-foreground text-sm">
              Set the settings that will be enabled for the authentication page.
            </span>

            <FormField
              control={form.control}
              name="canSelfSignUp"
              render={({ field }) => (
                <FormItem className="flex flex-col bg-gray-50 dark:bg-gray-900 p-2 rounded-sm">
                  <div className="flex items-center space-x-2">
                    <FormControl>
                      <Checkbox
                        checked={!!field.value}
                        onCheckedChange={field.onChange}
                        aria-labelledby="terms-label"
                      />
                    </FormControl>

                    <FormLabel className="text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                      User can self sign up
                    </FormLabel>
                  </div>

                  <FormDescription>
                    If this option is enabled, the user will be able to sign up
                    by himself.
                  </FormDescription>
                  <FormMessage></FormMessage>
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="canSelfForgotPass"
              render={({ field }) => (
                <FormItem className="flex flex-col bg-gray-50 dark:bg-gray-900 p-2 rounded-sm">
                  <div className="flex items-center space-x-2">
                    <FormControl>
                      <Checkbox
                        checked={!!field.value}
                        onCheckedChange={field.onChange}
                        aria-labelledby="terms-label"
                      />
                    </FormControl>

                    <FormLabel className="text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                      User can self forgot password
                    </FormLabel>
                  </div>

                  <FormDescription>
                    If this option is enabled, the user will be able to reset
                    his password by himself.
                  </FormDescription>
                  <FormMessage></FormMessage>
                </FormItem>
              )}
            />
          </div>

          <Button type="submit" disabled={isLoading} className="ml-auto w-fit">
            {isLoading ? "Creating Application..." : "Create Application"}
          </Button>
        </div>
      </form>
    </Form>
  );
}
