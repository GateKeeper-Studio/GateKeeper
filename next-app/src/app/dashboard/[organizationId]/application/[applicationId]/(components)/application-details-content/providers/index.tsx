import { useEffect, useState } from "react";

import { ProviderCard } from "./provider-card";
import { OAuthProvider, OAuthProviders } from "./providers";

import { IApplication } from "@/services/dashboard/get-application-by-id";

type Props = {
  application: IApplication | null;
};

export function Providers({ application }: Props) {
  const [oauthProviders, setOAuthProviders] = useState(OAuthProviders);

  useEffect(() => {
    if (!application) return;

    const authProviders = OAuthProviders.map((provider) => {
      const applicationProvider = application.oauthProviders.find(
        (providerItem) => providerItem.name === provider.id
      );

      if (applicationProvider) {
        provider.isEnabled = applicationProvider.isEnabled;
        provider.inputs = [
          ...provider.inputs.map((input) => {
            if (input.id.includes("client-id")) {
              input.value = applicationProvider.clientId;
            }

            if (input.id.includes("client-secret")) {
              input.value = applicationProvider.clientSecret;
            }

            if (input.id.includes("redirect-uri")) {
              input.value = applicationProvider.redirectUri;
            }

            return input;
          }),
        ];
      }

      return provider;
    });

    setOAuthProviders(authProviders);
  }, [application]);

  function handleChangeProvider(authProvider: OAuthProvider) {
    setOAuthProviders((state) =>
      state.map((item) => {
        if (item.id === authProvider.id) {
          return authProvider;
        }

        return item;
      })
    );
  }

  return (
    <div className="flex flex-wrap gap-4">
      {oauthProviders.map((provider) => (
        <ProviderCard
          key={provider.id}
          provider={provider}
          application={application}
          handleChangeProvider={handleChangeProvider}
        />
      ))}
    </div>
  );
}
