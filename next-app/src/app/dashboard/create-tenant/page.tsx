import { Metadata } from "next";

import { CreateTenantContent } from "./(components)/create-tenant-content";

type Props = {
  params: Promise<{
    tenantId: string;
  }>;
};

export const metadata: Metadata = {
  title: "Create Application - Application - GateKeeper",
};

export default async function CreateTenantPage({ params }: Props) {
  const { tenantId } = await params;

  return <CreateTenantContent />;
}
