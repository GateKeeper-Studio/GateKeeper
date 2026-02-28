import { ReactNode } from "react";

import { ApplicationAuthData } from "@/services/auth/get-application-auth-data";
import { Toaster } from "@/components/ui/sonner";
import { ThemeToggle } from "@/components/ui/theme-togle";

type Props = {
  children: ReactNode;
  page:
    | "sign-in"
    | "sign-up"
    | "forgot-password"
    | "confirm-email"
    | "one-time-password"
    | "change-password";
  application: ApplicationAuthData | null;
  termsAndConditionsEnabled?: boolean;
};

export function Background({
  children,
  application,
  termsAndConditionsEnabled = true,
}: Props) {
  return (
    <>
      <div className="relative flex h-screen flex-col items-center justify-center lg:max-w-none lg:grid lg:grid-cols-2 lg:px-0">
        <div className="bg-muted relative hidden h-screen flex-col p-10 text-white lg:flex dark:border-r">
          <div
            className="absolute inset-0 bg-cover"
            style={{
              backgroundImage:
                "url(https://images.unsplash.com/photo-1590069261209-f8e9b8642343?ixlib=rb-4.0.3&ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&auto=format&fit=crop&w=1376&q=80)",
            }}
          ></div>

          <div className="relative z-20 flex items-center text-lg font-medium">
            {application?.name}
          </div>

          <div className="relative z-20 mt-auto">
            <blockquote className="space-y-2">
              <p className="text-lg">
                &ldquo;This library has saved me countless hours of work and
                helped me deliver stunning designs to my clients faster than
                ever before. Highly recommended!&rdquo;
              </p>

              <footer className="text-sm">Sofia Davis</footer>
            </blockquote>
          </div>
        </div>

        <div className="flex flex-1 items-center relative justify-center h-full overflow-auto w-full">
          <div className="p-6 sm:p-8 w-full flex justify-center">
            <span className="absolute bottom-4 right-4">
              <ThemeToggle />
            </span>

            <div className="mx-auto flex w-full flex-col justify-center space-y-6 sm:w-[350px]">
              {children}

              {termsAndConditionsEnabled && (
                <p className="text-muted-foreground px-8 text-center text-sm">
                  By clicking continue, you agree to our
                  <a
                    href="/terms"
                    className="hover:text-primary underline underline-offset-4 mx-1"
                  >
                    Terms of Service
                  </a>
                  and
                  <a
                    href="/privacy"
                    className="hover:text-primary underline underline-offset-4 ml-1"
                  >
                    Privacy Policy
                  </a>
                  .
                </p>
              )}
            </div>
          </div>
        </div>
      </div>

      <Toaster richColors />
    </>
  );
}
