"use client";

import { useState } from "react";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { formatDate } from "@/lib/utils";

import { NewSecretDialog } from "./new-secret-dialog";
import { DeleteSecretDialog } from "./delete-secret-dialog";

import { IApplication } from "@/services/dashboard/get-application-by-id";

type Props = {
  application: IApplication | null;
};

export function SecretsSection({ application }: Props) {
  const [secrets, setSecrets] = useState(application?.secrets || []);

  function addSecret(newSecret: IApplication["secrets"][number]) {
    setSecrets((state) => [...state, newSecret]);
  }

  return (
    <Card className="w-full transition-all">
      <CardHeader>
        <CardTitle className="flex flex-wrap justify-between gap-4">
          Secrets
          <NewSecretDialog addSecret={addSecret} />
        </CardTitle>

        <CardDescription>
          Secrets are used to authenticate your application with the server.
          Keep them safe.
        </CardDescription>
      </CardHeader>

      <CardContent className="flex flex-col gap-y-4">
        {secrets.map((secret) => (
          <div className="flex items-center gap-4" key={secret.id}>
            <div className="space-y-1">
              <p className="text-sm font-medium leading-none">{secret.name}</p>
              <p className="text-muted-foreground">{secret.value}</p>
            </div>

            <div className="ml-auto text-sm">
              Expiration:{" "}
              <span className="text-md font-medium">
                {secret?.expirationDate
                  ? formatDate(new Date(secret.expirationDate))
                  : "Lifetime"}
              </span>
            </div>

            <Tooltip>
              <TooltipTrigger asChild>
                <DeleteSecretDialog
                  secret={secret}
                  removeSecret={() =>
                    setSecrets((state) =>
                      state.filter(
                        (secretState) => secretState.id !== secret.id
                      )
                    )
                  }
                />
              </TooltipTrigger>

              <TooltipContent>
                <p>Delete secret</p>
              </TooltipContent>
            </Tooltip>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}
