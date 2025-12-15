"use client";

import { z } from "zod";
import axios from "axios";
import Link from "next/link";
import { toast } from "sonner";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useParams, useRouter, useSearchParams } from "next/navigation";

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
import { Button } from "@/components/ui/button";
import { LoadingSpinner } from "@/components/ui/loading-spinner";

import { formSchema } from "./auth-schema";
import { zodResolver } from "@hookform/resolvers/zod";

import { authorizeApi } from "@/services/auth/authorize";
import { ApplicationAuthData } from "@/services/auth/get-application-auth-data";

import { ErrorAlert } from "@/components/error-alert";
import { EMfaType, loginApi } from "@/services/auth/login";

import { GithubLogo } from "@/app/dashboard/[organizationId]/application/[applicationId]/(components)/providers/github-logo";
import { GoogleLogo } from "@/app/dashboard/[organizationId]/application/[applicationId]/(components)/providers/google-logo";
import { api } from "@/services/base/gatekeeper-api";

type Props = {
  application: ApplicationAuthData | null;
};

export function AuthForm({ application }: Props) {
  const applicationId = useParams().applicationId as string;
  const searchParams = useSearchParams();
  const router = useRouter();

  const redirectUri = searchParams.get("redirect_uri") || "/";
  const codeChallengeMethod = searchParams.get("code_challenge_method") || "";
  const responseType = searchParams.get("response_type") || "";
  const scope = searchParams.get("scope") || "";
  const state = searchParams.get("state") || "";
  const codeChallenge = searchParams.get("code_challenge") || "";

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      email: "",
      password: "",
    },
  });

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const urlParams = new URLSearchParams({
    redirect_uri: redirectUri,
    response_type: responseType,
    scope,
    code_challenge_method: codeChallengeMethod,
    code_challenge: codeChallenge,
    state,
  });

  async function onSubmit(values: z.infer<typeof formSchema>) {
    setIsLoading(true);

    const [loginData, loginErr] = await loginApi({
      email: values.email.trim(),
      password: values.password.trim(),
      applicationId,
      redirectUri,
      responseType,
      scope,
      codeChallengeMethod,
      codeChallenge,
      state,
    });

    if (loginErr && loginErr.response?.data.title === "E-mail not confirmed") {
      window.location.href = `/auth/${applicationId}/confirm-email?${urlParams.toString()}&email=${
        values.email
      }&trying_to_sign_in=true`;
      return;
    }

    if (loginErr) {
      console.error(loginErr);
      setError(loginErr?.response?.data.message || "An error occurred");
      setIsLoading(false);
      setTimeout(() => setError(null), 6000);
      return;
    }

    if (!loginData) {
      setError("An error occurred");
      setIsLoading(false);
      setTimeout(() => setError(null), 6000);
      return;
    }

    setIsLoading(false);

    toast.success("You have successfully signed in");

    urlParams.append("mfa_auth_app_required", "true");
    urlParams.append("email", values.email);

    if (loginData.changePasswordCode) {
      urlParams.append("change_password_code", loginData.changePasswordCode);
    }

    if (loginData.userId) {
      urlParams.append("user_id", loginData.userId);
    }

    if (loginData.mfaType == EMfaType.MfaEmail) {
      router.push(
        `/auth/${applicationId}/mfa-mail?${urlParams.toString()}${
          loginData.mfaId ? `&mfa_id=${loginData.mfaId}` : ""
        }`
      );
      return;
    }

    if (loginData.mfaType == EMfaType.MfaApp) {
      router.push(
        `/auth/${applicationId}/mfa-app?${urlParams.toString()}${
          loginData.mfaId ? `&mfa_id=${loginData.mfaId}` : ""
        }`
      );
      return;
    }

    if (loginData.changePasswordCode && loginData.userId) {
      urlParams.append("session_code", loginData.sessionCode);

      router.push(
        `/auth/${applicationId}/update-password?${urlParams.toString()}${
          loginData.mfaId ? `&mfa_id=${loginData.mfaId}` : ""
        }`
      );
      return;
    }

    const [authorizeData, authorizeErr] = await authorizeApi({
      email: values.email.trim(),
      sessionCode: loginData.sessionCode,
      applicationId,
      redirectUri,
      responseType,
      scope,
      codeChallengeMethod,
      codeChallenge,
      state,
      mfaId: loginData.mfaId,
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

    const params = new URLSearchParams({
      code: authorizeData.authorizationCode,
      state,
      redirect_uri: redirectUri,
      client_id: applicationId,
    });

    window.location.href = `${redirectUri}?${params.toString()}`;
  }

  async function handleOAuthLogin(
    provider: ApplicationAuthData["oauthProviders"][number]
  ) {
    // const { data } = await axios.post<{ url: string }>(
    //   "/api/auth/external-login/github",
    //   {
    //     oauthProviderId: provider.id,
    //   }
    // );

    const { data } = await api.post<{ url: string }>(
      "/v1/auth/oauth-provider/github/login",
      {
        oauthProviderId: provider.id,
        clientCodeChallenge: codeChallenge,
        clientCodeChallengeMethod: codeChallengeMethod,
        clientRedirectUri: redirectUri,
        clientState: state,
        clientResponseType: responseType,
        clientScope: scope,
      }
    );

    window.location.href = data.url;
  }

  return (
    <div className="grid gap-4">
      {error && <ErrorAlert message={error} title="An error occurred..." />}

      <Form {...form}>
        <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-3">
          <FormField
            control={form.control}
            name="email"
            render={({ field }) => (
              <FormItem>
                <FormLabel>E-mail</FormLabel>
                <FormControl>
                  <Input
                    placeholder="example@email.com"
                    autoComplete="email"
                    type="email"
                    {...field}
                  />
                </FormControl>

                <FormDescription></FormDescription>
                <FormMessage />
              </FormItem>
            )}
          />

          <FormField
            control={form.control}
            name="password"
            render={({ field }) => (
              <FormItem>
                <FormLabel>Password</FormLabel>
                <FormControl>
                  <Input
                    placeholder="********"
                    autoComplete="new-password"
                    type="password"
                    {...field}
                  />
                </FormControl>

                <FormDescription></FormDescription>
                <FormMessage />
              </FormItem>
            )}
          />

          {(application?.canSelfSignUp || application?.canSelfForgotPass) && (
            <div className="flex items-center justify-between gap-2">
              {application?.canSelfForgotPass && (
                <Link
                  href={`/auth/${applicationId}/forgot-password?${urlParams.toString()}`}
                  className="text-md text-center hover:underline mx-auto"
                >
                  Forgot password?
                </Link>
              )}

              {application?.canSelfSignUp && (
                <Link
                  href={`/auth/${applicationId}/sign-up?${urlParams.toString()}`}
                  className="font-semibold text-md text-center hover:underline mx-auto"
                >
                  Create an account
                </Link>
              )}
            </div>
          )}

          <Button
            type="submit"
            disabled={isLoading}
            className="w-full relative"
          >
            {isLoading && <LoadingSpinner className="absolute left-4" />}
            Sign In with Email
          </Button>
        </form>
      </Form>

      {application && application.oauthProviders.length > 0 && (
        <>
          <div className="relative">
            <div className="absolute inset-0 flex items-center">
              <span className="w-full border-t"></span>
            </div>

            <div className="relative flex justify-center text-xs uppercase">
              <span className="bg-background text-muted-foreground px-2">
                {" "}
                Or continue with{" "}
              </span>
            </div>
          </div>

          {application.oauthProviders.map((provider) => (
            <div key={provider.id} className="flex flex-col gap-1">
              <Button
                variant="outline"
                type="button"
                className="flex items-center justify-center gap-2"
                disabled={isLoading}
                onClick={() => handleOAuthLogin(provider)}
                title={`Sign in with ${
                  provider.name.charAt(0).toUpperCase() + provider.name.slice(1)
                }`}
              >
                {provider.name === "github" && <GithubLogo size={24} />}
                {provider.name === "google" && <GoogleLogo size={24} />}

                {provider.name.charAt(0).toUpperCase() + provider.name.slice(1)}
              </Button>
            </div>
          ))}
        </>
      )}
    </div>
  );
}
