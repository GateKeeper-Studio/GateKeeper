import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";

import { IApplication } from "@/services/dashboard/get-application-by-id";

import { SecretsSection } from "./secrets-section";

type Props = {
  application: IApplication | null;
};

export function Overview({ application }: Props) {
  return (
    <section className="flex w-full flex-col gap-y-4">
      <Card className="w-full transition-all">
        <CardHeader>
          <CardTitle>Info</CardTitle>
          <CardDescription>Information about the application.</CardDescription>
        </CardHeader>

        <CardContent className="flex flex-wrap gap-x-8 gap-y-4">
          <div className="flex flex-col">
            <span className="text-md font-semibold">Application ID</span>
            <span className="text-sm">{application?.id}</span>
          </div>

          <div className="flex flex-col gap-1">
            <span className="text-md font-semibold">Multi Factor Auth</span>

            <div className="flex gap-3 flex-wrap">
              <div className="flex gap-2 items-center">
                <Checkbox
                  checked={application?.mfaAuthAppEnabled}
                  disabled
                  className="pointer-events-none"
                  aria-readonly="true"
                  id="mfa-auth-app"
                />

                <Label
                  className="pointer-events-none text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                  htmlFor="mfa-auth-app"
                >
                  Authenticator App
                </Label>
              </div>

              <div className="flex gap-2 items-center">
                <Checkbox
                  checked={application?.mfaEmailEnabled}
                  disabled
                  className="pointer-events-none"
                  id="mfa-email"
                  aria-readonly="true"
                />
                <Label
                  className="pointer-events-none text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                  htmlFor="mfa-email"
                >
                  E-mail
                </Label>
              </div>
            </div>
          </div>

          <div className="flex flex-col gap-1">
            <span className="text-md font-semibold">Authentication Page</span>

            <div className="flex gap-3 flex-wrap">
              <div className="flex gap-2 items-center">
                <Checkbox
                  checked={application?.canSelfSignUp}
                  disabled
                  className="pointer-events-none"
                  aria-readonly="true"
                  id="mfa-auth-app"
                />

                <Label
                  className="pointer-events-none text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                  htmlFor="mfa-auth-app"
                >
                  User can self sign up
                </Label>
              </div>

              <div className="flex gap-2 items-center">
                <Checkbox
                  checked={application?.canSelfForgotPass}
                  disabled
                  className="pointer-events-none"
                  id="mfa-email"
                  aria-readonly="true"
                />
                <Label
                  className="pointer-events-none text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                  htmlFor="mfa-email"
                >
                  User can self forgot password
                </Label>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <SecretsSection application={application} />
    </section>
  );
}
