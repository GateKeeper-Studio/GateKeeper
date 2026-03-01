"use client";

import { toast } from "sonner";
import { useParams } from "next/navigation";
import { useEffect, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button, buttonVariants } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { Switch } from "@/components/ui/switch";

import { OAuthProvider } from "./providers";

import { IApplication } from "@/services/dashboard/get-application-by-id";
import { configureApplicationOauthProviderApi } from "@/services/dashboard/configure-application-oauth-provider";

type Props = {
  provider: OAuthProvider;
  application: IApplication | null;
  handleChangeProvider: (authProvider: OAuthProvider) => void;
};

export function ProviderCard({
  provider,
  application,
  handleChangeProvider,
}: Props) {
  const organizationId = useParams().organizationId as string;

  const [drafts, setDrafts] = useState<OAuthProvider["inputs"]>(
    provider.inputs
  );

  const [draftIsEnabled, setDraftIsEnabled] = useState(provider.isEnabled);

  useEffect(() => {
    setDrafts(provider.inputs);
    setDraftIsEnabled(provider.isEnabled);
  }, [provider.inputs, provider.isEnabled]);

  function setDraftSpecificInput(index: number, value: string) {
    setDrafts((prev) => {
      prev[index].value = value;

      return [...prev];
    });
  }

  const [isSheetOpen, setIsSheetOpen] = useState(false);

  const isConfigured = provider.inputs.every((input) => input.value);

  async function handleSaveChanges() {
    if (!application) {
      console.error("Application is not defined");
      toast.error("Application is not defined");
      return;
    }

    if (!organizationId) {
      console.error("Organization ID is not defined");
      toast.error("Organization ID is not defined");
      return;
    }

    const clientId =
      drafts.find((input) => input.id.includes("client-id"))?.value || "";
    const clientSecret =
      drafts.find((input) => input.id.includes("client-secret"))?.value || "";
    const redirectUri =
      drafts.find((input) => input.id.includes("redirect-uri"))?.value || "";

    const [, err] = await configureApplicationOauthProviderApi(
      {
        applicationId: application?.id,
        clientId,
        clientSecret,
        redirectUri,
        enabled: draftIsEnabled,
        name: provider.id,
        organizationId,
      },
      { accessToken: "fake-token" }
    );

    if (err) {
      console.error(err);
      toast.error("An error occurred while configuring the provider.");
      return;
    }

    toast.success(`${provider.name} provider configured successfully!`);

    handleChangeProvider({
      ...provider,
      isEnabled: draftIsEnabled,
      inputs: drafts,
    });
    setIsSheetOpen(false);
  }

  return (
    <Sheet open={isSheetOpen} onOpenChange={setIsSheetOpen}>
      <SheetTrigger>
        <Card className="transition-all hover:scale-[1.01] hover:cursor-pointer hover:shadow-lg flex overflow-hidden">
          <div className="flex items-center justify-center p-4 bg-gray-50 dark:bg-gray-900">
            <provider.logo />
          </div>

          <div className="flex flex-col">
            <CardHeader>
              <CardTitle className="flex flex-wrap justify-between gap-4">
                {provider.name}
              </CardTitle>

              <CardDescription className="text-left">
                {provider.description}
              </CardDescription>
            </CardHeader>

            <CardContent className="flex gap-1">
              {isConfigured && (
                <Badge
                  variant="default"
                  className="w-fit bg-green-500 text-white"
                >
                  Configured
                </Badge>
              )}

              {!isConfigured && (
                <Badge variant="outline" className="w-fit">
                  Not configured
                </Badge>
              )}

              {provider.isEnabled && (
                <Badge variant="default" className="w-fit">
                  Enabled
                </Badge>
              )}

              {!provider.isEnabled && (
                <Badge variant="secondary" className="w-fit">
                  Disabled
                </Badge>
              )}
            </CardContent>
          </div>
        </Card>
      </SheetTrigger>

      <SheetContent side="right">
        <SheetHeader>
          <SheetTitle>Configure {provider.name} Provider</SheetTitle>

          <SheetDescription>
            Make changes to your authentication provider configuration. Then
            click &quot;Save changes&quot; to apply them.
          </SheetDescription>
        </SheetHeader>

        <div className="flex flex-col gap-4 px-4">
          {provider.inputs.map((input, index) => (
            <div className="flex flex-col gap-2" key={input.id}>
              <Label htmlFor="client-id">
                {input.label}
                {input.required && <span className="text-red-500 ml-1">*</span>}
              </Label>

              <Input
                id={input.id}
                value={input.value ?? ""}
                type={input.type}
                placeholder={input.placeholder}
                required={input.required}
                className="col-span-3"
                onChange={(e) => setDraftSpecificInput(index, e.target.value)}
              />
            </div>
          ))}

          <div className="flex items-center space-x-2">
            <Switch
              id="provider-enabled"
              checked={draftIsEnabled}
              onCheckedChange={() => setDraftIsEnabled(!draftIsEnabled)}
            />

            <Label htmlFor="provider-enabled">
              {draftIsEnabled ? "Enabled" : "Disabled"}
            </Label>
          </div>
        </div>

        <SheetFooter>
          <Button onClick={handleSaveChanges}>Save changes</Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
