"use client";

import { useParams } from "next/navigation";
import { createContext, useContext, type ReactNode } from "react";

import { IApplication } from "@/services/dashboard/use-application-by-id-swr";
import { useApplicationByIdSWR } from "@/services/dashboard/use-application-by-id-swr";

import { ErrorAlert } from "@/components/error-alert";

type ApplicationContextType = {
  application: IApplication | null;
  isLoading: boolean;
};

const ApplicationContext = createContext({} as ApplicationContextType);

export function ApplicationContextProvider({
  children,
}: {
  children: ReactNode;
}) {
  const { organizationId, applicationId } = useParams() as {
    organizationId: string;
    applicationId: string;
  };

  const {
    data: application,
    error,
    isLoading,
  } = useApplicationByIdSWR(
    { applicationId, organizationId },
    { accessToken: "" },
  );

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
    <ApplicationContext.Provider
      value={{
        application: application || null,
        isLoading,
      }}
    >
      {children}
    </ApplicationContext.Provider>
  );
}

export const useApplicationContext = () => {
  return useContext(ApplicationContext);
};
