"use client";

import Link from "next/link";
import { AlertCircle } from "lucide-react";

import { Skeleton } from "@/components/ui/skeleton";
import { Card, CardHeader, CardTitle } from "@/components/ui/card";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

import { APIError } from "@/types/service-options";
import { useTenantsSWR } from "@/services/dashboard/use-tenants-swr";

export function TenantsList() {
  const { data, isLoading, error } = useTenantsSWR({ accessToken: "" });

  const err = error as APIError;

  if (isLoading) {
    <>
      <Skeleton className="h-[133px] max-w-[400px] flex-1" />
      <Skeleton className="h-[133px] max-w-[400px] flex-1" />
    </>;
  }

  if (err) {
    return (
      <Alert variant="destructive" className="bg-red-500/10">
        <AlertCircle className="h-4 w-4" />
        <AlertTitle>An error occurred...</AlertTitle>
        <AlertDescription>{err.response?.data.message}</AlertDescription>
      </Alert>
    );
  }

  if (data?.length === 0) {
    return (
      <Alert variant="default" className="bg-yellow-500/10">
        <AlertCircle className="h-4 w-4" />
        <AlertTitle>No tenants found...</AlertTitle>

        <AlertDescription>
          You don&apos;t have any tenants yet. Create one to get started.
        </AlertDescription>
      </Alert>
    );
  }

  return (
    data?.map((tenant) => (
      <Link
        className="min-w-[400px] flex-1 max-w-[400px]"
        key={tenant.id}
        href={`/dashboard/${tenant.id}/application`}
      >
        <Card className="flex-1 transition-all hover:scale-[1.01] hover:cursor-pointer hover:shadow-lg">
          <CardHeader className="p-4">
            <div className="flex flex-col gap-4">
              <div className="min-h-[120px] w-full rounded-lg bg-black/10"></div>
              <CardTitle>{tenant.name}</CardTitle>
            </div>
          </CardHeader>
        </Card>
      </Link>
    )) || []
  );
}
