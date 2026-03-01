"use client";

import { useParams } from "next/navigation";
import { createContext, useContext, useEffect, type ReactNode } from "react";

import { ErrorAlert } from "@/components/error-alert";
import {
  useOrganizationsSWR,
  type Organization,
} from "@/services/dashboard/use-organizations-swr";

type OrganizationsContextType = {
  organizations: Organization[];
  selectedOrganization: Organization | null;
  isLoading: boolean;
};

const OrganizationsContext = createContext({} as OrganizationsContextType);

export function OrganizationsContextProvider({
  children,
}: {
  children: ReactNode;
}) {
  const { organizationId } = useParams() as {
    organizationId: string;
  };

  const {
    data: organization,
    error,
    isLoading,
  } = useOrganizationsSWR({ accessToken: "" });

  if (error) {
    return (
      <div className="flex m-4 w-full">
        <ErrorAlert
          message={
            error.response?.data.message || "Failed on trying to fetch project"
          }
          title={error.response?.data.title || "An Error Occurred"}
        />
      </div>
    );
  }

  return (
    <OrganizationsContext.Provider
      value={{
        organizations: organization || [],
        selectedOrganization:
          organization?.find((org) => org.id === organizationId) || null,
        isLoading,
      }}
    >
      {children}
    </OrganizationsContext.Provider>
  );
}

export const useOrganizationsContext = () => {
  return useContext(OrganizationsContext);
};
