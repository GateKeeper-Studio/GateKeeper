"use client";

import { Checkbox } from "@/components/ui/checkbox";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  FormControl,
  FormField,
  FormItem,
  FormLabel,
} from "@/components/ui/form";

import { FormType } from "./user-detail-form";

type Props = {
  form: FormType;
};

export function MultiFactorAuthForm({ form }: Props) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-sm font-medium"> Multi Factor Authentication </span>

      <span className="text-muted-foreground my-2 text-sm">
        Choose the methods that will be used for multi factor authentication.
      </span>

      <div className="flex items-center space-x-2">
        <FormField
          control={form.control}
          name="hasMfaEmailEnabled"
          render={({ field }) => (
            <FormItem className="flex items-center space-x-2">
              <FormControl>
                <Checkbox
                  checked={!!field.value}
                  onCheckedChange={field.onChange}
                  aria-labelledby="terms-label"
                  id="e-mail-mfa"
                />
              </FormControl>

              <Tooltip>
                <TooltipTrigger type="button">
                  <FormLabel
                    htmlFor="e-mail-mfa"
                    className="text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                  >
                    E-mail
                  </FormLabel>
                </TooltipTrigger>

                <TooltipContent>
                  Send a verification code to the user&apos;s email address.
                </TooltipContent>
              </Tooltip>
            </FormItem>
          )}
        />
      </div>

      <div className="flex items-center space-x-2">
        <FormField
          control={form.control}
          name="hasMfaAuthAppEnabled"
          render={({ field }) => (
            <FormItem className="flex items-center space-x-2">
              <FormControl>
                <Checkbox
                  checked={!!field.value}
                  onCheckedChange={field.onChange}
                  aria-labelledby="terms-label"
                  id="auth-app-mfa-2"
                />
              </FormControl>

              <Tooltip>
                <TooltipTrigger type="button">
                  <FormLabel
                    htmlFor="auth-app-mfa-2"
                    className="text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                  >
                    Authenticator App (Microsoft, Google, etc)
                  </FormLabel>
                </TooltipTrigger>

                <TooltipContent>
                  Use an authenticator app to generate a verification code.
                </TooltipContent>
              </Tooltip>
            </FormItem>
          )}
        />
      </div>
    </div>
  );
}
