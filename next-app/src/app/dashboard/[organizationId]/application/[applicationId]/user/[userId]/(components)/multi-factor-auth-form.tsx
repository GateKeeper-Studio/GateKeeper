"use client";

import { useState } from "react";
import { toast } from "sonner";
import { startRegistration } from "@simplewebauthn/browser";

import { FormType } from "./user-detail-form";
import { Button } from "@/components/ui/button";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";

import { beginWebAuthnRegistrationApi } from "@/services/auth/begin-webauthn-registration";
import { verifyWebAuthnRegistrationApi } from "@/services/auth/verify-webauthn-registration";

type Props = {
  form: FormType;
  isEditEnabled: boolean;
  userId: string;
  applicationId: string;
};

export function MultiFactorAuthForm({
  isEditEnabled,
  form,
  userId,
  applicationId,
}: Props) {
  const [isRegisteringPasskey, setIsRegisteringPasskey] = useState(false);

  async function handleRegisterPasskey() {
    setIsRegisteringPasskey(true);

    const [beginData, beginErr] = await beginWebAuthnRegistrationApi({
      userId,
      applicationId,
    });

    if (beginErr || !beginData) {
      toast.error(
        beginErr?.response?.data?.message ||
          "Failed to start passkey registration.",
      );
      setIsRegisteringPasskey(false);
      return;
    }

    let registrationResponse;

    try {
      const rawOptions = beginData.options as { publicKey?: unknown } | null;
      const registrationOptions = rawOptions?.publicKey ?? rawOptions;

      registrationResponse = await startRegistration({
        optionsJSON: registrationOptions as Parameters<
          typeof startRegistration
        >[0]["optionsJSON"],
      });
    } catch (err: unknown) {
      const message =
        err instanceof Error
          ? err.message
          : "Passkey registration was cancelled or failed.";

      toast.error(message);

      setIsRegisteringPasskey(false);
      return;
    }

    const [, verifyErr] = await verifyWebAuthnRegistrationApi({
      userId,
      applicationId,
      sessionId: beginData.sessionId,
      credentialData: registrationResponse,
    });

    if (verifyErr) {
      toast.error(
        verifyErr?.response?.data?.message ||
          "Failed to verify passkey registration.",
      );
      setIsRegisteringPasskey(false);
      return;
    }

    form.setValue("isMfaWebauthnConfigured", true);
    toast.success("Passkey registered successfully.");
    setIsRegisteringPasskey(false);
  }

  return (
    <div className="flex flex-col gap-1">
      <FormField
        control={form.control}
        name="preferred2FAMethod"
        render={({ field }) => (
          <FormItem>
            <FormLabel>Preferred MFA Method</FormLabel>
            <Select
              onValueChange={field.onChange}
              defaultValue={field.value || "Nenhum"}
              disabled={!isEditEnabled}
            >
              <FormControl>
                <SelectTrigger className="min-w-[10rem]">
                  <SelectValue placeholder="Select a MFA method to display" />
                </SelectTrigger>
              </FormControl>

              <SelectContent>
                <SelectItem value="email">E-mail</SelectItem>
                <SelectItem value="totp">Authenticator App</SelectItem>
                <SelectItem value="webauthn">Passkey (WebAuthn)</SelectItem>
              </SelectContent>
            </Select>

            <FormDescription>
              Set your preferred method to use for multi-factor authentication
              when signing into the application.
            </FormDescription>
            <FormMessage />
          </FormItem>
        )}
      />

      <ul className="flex flex-col gap-1 mt-3">
        <li className="flex justify-between items-center w-full">
          <div className="flex gap-4 items-center">
            <Tooltip>
              <TooltipTrigger type="button">
                <span className="text-muted-background font-semibold">
                  E-mail
                </span>
              </TooltipTrigger>

              <TooltipContent>
                Send a verification code to the user&apos;s email address.
              </TooltipContent>
            </Tooltip>

            {form.getValues("isMfaEmailConfigured") ? (
              <span className="text-sm text-green bg-green-300/30 px-3 rounded-full border border-green-300 text-green-900 dark:text-green-300">
                Configured
              </span>
            ) : (
              <span className="text-sm bg-red-300/30 px-3 rounded-full border border-red-300 text-red-900 dark:text-red-300">
                Not Configured
              </span>
            )}
          </div>

          {isEditEnabled && (
            <Button variant="outline" className="font-semibold text-red-500">
              Disable
            </Button>
          )}
        </li>

        <li className="flex justify-between items-center w-full">
          <div className="flex gap-4 items-center">
            <Tooltip>
              <TooltipTrigger type="button">
                <span className="text-muted-background font-semibold">
                  Authenticator App
                </span>
              </TooltipTrigger>

              <TooltipContent>
                Use an authenticator app to generate a verification code.
              </TooltipContent>
            </Tooltip>

            {form.getValues("IsMfaAuthAppConfigured") ? (
              <span className="text-sm text-green bg-green-300/30 px-3 rounded-full border border-green-300 text-green-900 dark:text-green-300">
                Configured
              </span>
            ) : (
              <span className="text-sm bg-red-300/30 px-3 rounded-full border border-red-300 text-red-900 dark:text-red-300">
                Not Configured
              </span>
            )}
          </div>

          {isEditEnabled && (
            <Button variant="outline" className="font-semibold text-red-500">
              Disable
            </Button>
          )}
        </li>

        <li className="flex justify-between items-center w-full">
          <div className="flex gap-4 items-center">
            <Tooltip>
              <TooltipTrigger type="button">
                <span className="text-muted-background font-semibold">
                  Passkey (WebAuthn)
                </span>
              </TooltipTrigger>

              <TooltipContent>
                Use a passkey or security key for passwordless authentication.
              </TooltipContent>
            </Tooltip>

            {form.getValues("isMfaWebauthnConfigured") ? (
              <span className="text-sm text-green bg-green-300/30 px-3 rounded-full border border-green-300 text-green-900 dark:text-green-300">
                Configured
              </span>
            ) : (
              <span className="text-sm bg-red-300/30 px-3 rounded-full border border-red-300 text-red-900 dark:text-red-300">
                Not Configured
              </span>
            )}
          </div>

          <Button
            type="button"
            variant="outline"
            className="font-semibold"
            disabled={isRegisteringPasskey}
            onClick={handleRegisterPasskey}
          >
            {isRegisteringPasskey && <LoadingSpinner className="mr-2" />}
            Register Passkey
          </Button>
        </li>
      </ul>
    </div>
  );
}
