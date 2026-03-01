import Link from "next/link";
import { Metadata } from "next";
import { ChevronLeft } from "lucide-react";

import { CreateApplicationForm } from "./(components)/create-application-content/create-application-form";
import { DashboardHeader } from "@/components/dashboard-header";
import { CreateApplicationContent } from "./(components)/create-application-content";

type Props = {
  params: Promise<{
    organizationId: string;
  }>;
};

export const metadata: Metadata = {
  title: "Create Application - Application - GateKeeper",
};

export default async function CreateApplicationPage({ params }: Props) {
  const { organizationId } = await params;

  return <CreateApplicationContent />;
}
