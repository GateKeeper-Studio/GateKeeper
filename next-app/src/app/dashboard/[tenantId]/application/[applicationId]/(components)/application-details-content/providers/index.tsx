import { useEffect, useState } from "react";

import { ProviderCard } from "./provider-card";
import { OAuthProvider, OAuthProviders } from "./providers";

import { IApplication } from "@/services/dashboard/get-application-by-id";
import { SectionTitle } from "@/components/section-title";
import { useProvidersDataByApplicationIdSWR } from "@/services/dashboard/use-providers-data-by-application-id-swr";
import { useTenantsContext } from "@/app/dashboard/(contexts)/tenants-context-provider";
import { ErrorAlert } from "@/components/error-alert";

type Props = {
  application: IApplication | null;
};

export function Providers({ application }: Props) {
  const [oauthProviders, setOAuthProviders] = useState(OAuthProviders);
  const { selectedTenant } = useTenantsContext();

  const { data, error, isLoading, mutate } = useProvidersDataByApplicationIdSWR(
    {
      tenantId: selectedTenant?.id || "",
      applicationId: application?.id || "",
    },
    { accessToken: "" },
  );

  if (error) {
    return (
      <div className="flex m-4 w-full">
        <ErrorAlert
          message={
            error.response?.data.message || "Failed on trying to fetch users"
          }
          title={error.response?.data.title || "An Error Occurred"}
        />
      </div>
    );
  }

  useEffect(() => {
    if (!application) return;

    const authProviders = OAuthProviders.map((provider) => {
      const applicationProvider = data?.find(
        (providerItem) => providerItem.name === provider.id,
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
  }, [application, data]);

  function handleChangeProvider(authProvider: OAuthProvider) {
    setOAuthProviders((state) =>
      state.map((item) => {
        if (item.id === authProvider.id) {
          return authProvider;
        }

        return item;
      }),
    );
  }

  return (
    <section className="flex flex-col gap-4 w-full">
      <SectionTitle>OAuth Providers</SectionTitle>

      <span className="text-sm tracking-tight text-gray-600 dark:text-gray-300">
        OAuth providers allow you to enable third-party authentication for your
        application. You can configure providers like Google, GitHub, and more
        to allow users to sign in using their existing accounts.
      </span>

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
    </section>
  );
}
