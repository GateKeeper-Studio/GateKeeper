"use client";

import { z } from "zod";
import { toast } from "sonner";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useParams, useRouter, useSearchParams } from "next/navigation";

import {
  FormControl,
  Form,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Button } from "@/components/ui/button";

import { formSchema } from "./auth-schema";
import { zodResolver } from "@hookform/resolvers/zod";
import {
  InputOTP,
  InputOTPGroup,
  InputOTPSeparator,
  InputOTPSlot,
} from "@/components/ui/input-otp";
import { LoadingSpinner } from "@/components/ui/loading-spinner";

import { ErrorAlert } from "@/components/error-alert";

import { authorizeApi } from "@/services/auth/authorize";
import { verifyAppMfaApi } from "@/services/auth/verify-app-mfa";
import { MfaModal } from "../../(components)/mfa-modal";

export function AuthForm() {
  const applicationId = useParams().applicationId as string;
  const searchParams = useSearchParams();
  const router = useRouter();

  const redirectUri = searchParams.get("redirect_uri") || "/";
  const codeChallengeMethod = searchParams.get("code_challenge_method") || "";
  const responseType = searchParams.get("response_type") || "";
  const scope = searchParams.get("scope") || "";
  const state = searchParams.get("state") || "";
  const email = searchParams.get("email") || "";
  const codeChallenge = searchParams.get("code_challenge") || "";

  const changePasswordCode = searchParams.get("change_password_code") || "";
  const mfaAuthAppRequired = searchParams.get("mfa_auth_app_required") || "";
  const userId = searchParams.get("user_id") || "";
  const mfaId = searchParams.get("mfa_id") || "";
  const nonce = searchParams.get("nonce") || "";

  const urlParams = new URLSearchParams({
    redirect_uri: redirectUri,
    response_type: responseType,
    scope,
    code_challenge_method: codeChallengeMethod,
    code_challenge: codeChallenge,
    state,
    mfa_auth_app_required: mfaAuthAppRequired,
    email,
    mfa_id: mfaId,
    ...(nonce ? { nonce } : {}),
  });

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      code: "",
    },
  });

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(values: z.infer<typeof formSchema>) {
    setIsLoading(true);

    const [verifyMfaData, verifyMfaErr] = await verifyAppMfaApi({
      email: email.trim(),
      applicationId,
      code: values.code,
      mfaId,
    });

    if (verifyMfaErr) {
      console.error(verifyMfaErr);
      setError(verifyMfaErr?.response?.data.message || "An error occurred");
      setIsLoading(false);
      setTimeout(() => setError(null), 6000);
      return;
    }

    if (!verifyMfaData) {
      setError("An error occurred");
      setIsLoading(false);
      setTimeout(() => setError(null), 6000);
      return;
    }

    if (changePasswordCode && userId) {
      urlParams.append("session_code", verifyMfaData.sessionCode);
      urlParams.append("change_password_code", changePasswordCode);
      urlParams.append("user_id", userId);

      router.push(
        `/auth/${applicationId}/update-password?${urlParams.toString()}`,
      );
      return;
    }

    const [authorizeData, authorizeErr] = await authorizeApi({
      email: email.trim(),
      sessionCode: verifyMfaData.sessionCode,
      applicationId,
      redirectUri,
      responseType,
      scope,
      codeChallengeMethod,
      codeChallenge,
      state,
      nonce: nonce || undefined,
    });

    if (authorizeErr) {
      console.error(authorizeErr);
      setError(authorizeErr?.response?.data.message || "An error occurred");
      setIsLoading(false);
      setTimeout(() => setError(null), 6000);
      return;
    }

    if (!authorizeData) {
      setError("An error occurred");
      setIsLoading(false);
      setTimeout(() => setError(null), 6000);
      return;
    }

    setIsLoading(false);

    toast.success("You have successfully signed in");

    window.location.href = `${redirectUri}?code=${authorizeData.authorizationCode}&state=${state}&redirect_uri=${redirectUri}&client_id=${applicationId}`;
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

      <MfaModal />
    </div>
  );
}
