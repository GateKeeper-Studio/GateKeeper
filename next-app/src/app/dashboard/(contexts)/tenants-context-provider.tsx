"use client";

import { useParams } from "next/navigation";
import { createContext, useContext, useEffect, type ReactNode } from "react";

import { ErrorAlert } from "@/components/error-alert";
import {
  useTenantsSWR,
  type Tenant,
} from "@/services/dashboard/use-tenants-swr";

type TenantsContextType = {
  tenants: Tenant[];
  selectedTenant: Tenant | null;
  isLoading: boolean;
};

const TenantsContext = createContext({} as TenantsContextType);

export function TenantsContextProvider({
  children,
}: {
  children: ReactNode;
}) {
  const { tenantId } = useParams() as {
    tenantId: string;
  };

  const {
    data: tenant,
    error,
    isLoading,
  } = useTenantsSWR({ accessToken: "" });

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
    <TenantsContext.Provider
      value={{
        tenants: tenant || [],
        selectedTenant:
          tenant?.find((org) => org.id === tenantId) || null,
        isLoading,
      }}
    >
      {children}
    </TenantsContext.Provider>
  );
}

export const useTenantsContext = () => {
  return useContext(TenantsContext);
};
