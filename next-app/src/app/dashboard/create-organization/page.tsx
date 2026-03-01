import { Metadata } from "next";

import { CreateOrganizationContent } from "./(components)/create-organization-content";

type Props = {
  params: Promise<{
    organizationId: string;
  }>;
};

export const metadata: Metadata = {
  title: "Create Application - Application - GateKeeper",
};

export default async function CreateOrganizationPage({ params }: Props) {
  const { organizationId } = await params;

  return <CreateOrganizationContent />;
}
