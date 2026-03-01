"use client";

import Link from "next/link";
import { ChevronLeft } from "lucide-react";

import {
  Card,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

import { useApplicationsSWR } from "@/services/dashboard/use-applications-swr";

type Props = {
  organizationId: string;
  initialName: string;
  initialDescription: string;
};

export function ViewOrganizationContent({
  organizationId,
  initialName,
  initialDescription,
}: Props) {
  const { data: applications } = useApplicationsSWR(
    { organizationId },
    { accessToken: "fake-token" },
  );

  return (
    <main className="flex flex-col p-4">
      <Link
        href="/dashboard/organizations"
        className="w-fit text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:dark:text-gray-500 hover:text-gray-800 hover:underline"
      >
        <ChevronLeft size={24} />
        Go back to organizations list
      </Link>

      <h2 className="text-3xl font-bold tracking-tight">{initialName}</h2>

      <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
        {initialDescription}
      </span>

      <div className="mt-4 flex flex-wrap gap-4">
        {applications?.map((item) => (
          <Link
            key={item.id}
            href={`/dashboard/${organizationId}/application/${item.id}`}
          >
            <Card className="w-[calc(33.333%-8px)] min-w-[400px] transition-all hover:scale-[1.01] hover:cursor-pointer hover:shadow-lg">
              <CardHeader>
                <CardTitle>{item.name}</CardTitle>
                <CardDescription className="line-clamp-4">
                  {item.description}
                </CardDescription>
              </CardHeader>

              <CardFooter className="mt-3">
                {item.badges.map((badge, i) => (
                  <Badge key={i} variant="outline">
                    {badge}
                  </Badge>
                ))}
              </CardFooter>
            </Card>
          </Link>
        ))}
      </div>
    </main>
  );
}
