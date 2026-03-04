"use client";

import { z } from "zod";
import { toast } from "sonner";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useParams, useSearchParams } from "next/navigation";

import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Button } from "@/components/ui/button";
import {
  InputOTP,
  InputOTPGroup,
  InputOTPSeparator,
  InputOTPSlot,
} from "@/components/ui/input-otp";
import { ErrorAlert } from "@/components/error-alert";
import { LoadingSpinner } from "@/components/ui/loading-spinner";

import { formSchema } from "./auth-schema";
import { zodResolver } from "@hookform/resolvers/zod";
import { confirmEmailApi } from "@/services/auth/confirm-email";

export function AuthForm() {
  const applicationId = useParams().applicationId;
  const searchParams = useSearchParams();

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const scope = searchParams.get("scope") || "";
  const state = searchParams.get("state") || "";
  const email = searchParams.get("email") || "/";
  const redirectUri = searchParams.get("redirect_uri") || "/";
  const responseType = searchParams.get("response_type") || "";
  const codeChallenge = searchParams.get("code_challenge") || "";
  const codeChallengeMethod = searchParams.get("code_challenge_method") || "";
  const nonce = searchParams.get("nonce") || "";

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      code: "",
    },
  });

  async function onSubmit(values: z.infer<typeof formSchema>) {
    setIsLoading(true);

    const [data, err] = await confirmEmailApi({
      applicationId: (applicationId as string) || "",
      token: values.code,
      email,
      codeChallengeMethod,
      responseType,
      scope,
      state,
      codeChallenge,
      redirectUri,
      nonce,
    });

    if (err) {
      toast.error(err.response?.data.message || err.message);
      console.error(err);
      return;
    }

    if (!data) {
      setError("An error occurred");
      setIsLoading(false);
      setTimeout(() => setError(null), 6000);
      return;
    }

    setIsLoading(false);

    toast.success("Email confirmed!");

    // Redirect to the redirect_uri
    window.location.href = `${redirectUri}?code=${data.authorizationCode}&state=${state}&redirect_uri=${redirectUri}&client_id=${applicationId}`;
  }

  return (
    <div className="grid gap-4">
      {error && <ErrorAlert message={error} title="An error occurred..." />}

      <Form {...form}>
        <form
          onSubmit={form.handleSubmit(onSubmit)}
          className="space-y-3 w-full items-center flex flex-col"
        >
          <FormField
            control={form.control}
            name="code"
            render={({ field }) => (
              <FormItem>
                <FormLabel className="sr-only">Code</FormLabel>
                <FormControl>
                  <InputOTP
                    maxLength={6}
                    onChange={field.onChange}
                    value={field.value}
                  >
                    <InputOTPGroup>
                      <InputOTPSlot index={0} />
                      <InputOTPSlot index={1} />
                      <InputOTPSlot index={2} />
                    </InputOTPGroup>

                    <InputOTPSeparator />

                    <InputOTPGroup>
                      <InputOTPSlot index={3} />
                      <InputOTPSlot index={4} />
                      <InputOTPSlot index={5} />
                    </InputOTPGroup>
                  </InputOTP>
                </FormControl>

                <FormDescription></FormDescription>
                <FormMessage />
              </FormItem>
            )}
          />

          <Button
            type="submit"
            disabled={isLoading}
            className="w-full relative"
          >
            {isLoading && <LoadingSpinner className="absolute left-4" />}
            Confirm Code
          </Button>
        </form>
      </Form>
    </div>
  );
}
