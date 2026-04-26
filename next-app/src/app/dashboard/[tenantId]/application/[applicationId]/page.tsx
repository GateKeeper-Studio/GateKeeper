import { getApplicationByIdService } from "@/services/dashboard/get-application-by-id";

import { ApplicationDetailsContent } from "./(components)/application-details-content";

type Props = {
  params: Promise<{
    applicationId: string;
    tenantId: string;
  }>;
};

export async function generateMetadata({ params }: Props) {
  const { applicationId, tenantId } = await params;

  const [application, err] = await getApplicationByIdService(
    { applicationId, tenantId },
    { accessToken: "" },
  );

  if (err) {
    return {
      title: "Application - GateKeeper",
    };
  }

  return {
    title: `${application?.name} - Application - GateKeeper`,
  };
}

export default async function ApplicationDetailPage({ params }: Props) {
  const { applicationId, tenantId } = await params;

  return (
    <ApplicationDetailsContent
      applicationId={applicationId}
      tenantId={tenantId}
    />
  );
}
