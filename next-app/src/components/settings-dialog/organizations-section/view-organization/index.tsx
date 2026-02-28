import { Breadcrumbs } from "@/components/dashboard-header/bread-crumbs";

import Link from "next/link";
import { ChevronLeft } from "lucide-react";
import { useSearchParams } from "next/navigation";

import { useOrganizationByIdSWR } from "@/services/settings/use-organization-by-id-swr";
import { useApplicationsSWR } from "@/services/dashboard/use-applications-swr";
import {
  Card,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

import { OrganizationsPages } from "..";

type Props = {
  setPage: (page: OrganizationsPages) => void;
};

export function ViewOrganization({ setPage }: Props) {
  const searchParams = useSearchParams();

  const { data: organization } = useOrganizationByIdSWR(
    { id: searchParams.get("organizationId") || "" },
    { accessToken: "fake-token" },
  );

  const { data: applications } = useApplicationsSWR(
    {
      organizationId: organization?.id,
    },
    { accessToken: "fake-token" },
  );

  return (
    <main className="flex flex-col p-4">
      <Breadcrumbs
        items={[{ name: "Settings" }, { name: "Organizations" }]}
        disableSideBar
      />

      <button
        type="button"
        onClick={() => setPage("default")}
        className="mt-4 w-fit text-md mb-4 flex items-center gap-2 text-gray-600 dark:text-gray-300 hover:dark:text-gray-500 hover:text-gray-800 hover:underline"
      >
        <ChevronLeft size={24} />
        Go back to organizations list
      </button>

      <h2 className="text-3xl font-bold tracking-tight">
        {organization?.name}
      </h2>

      <span className="mt-3 text-sm tracking-tight text-gray-600 dark:text-gray-300">
        {organization?.description}
      </span>

      <div className="mt-4 flex gap-4">
        {applications?.map((item) => (
          <Link
            key={item.id}
            href={`/dashboard/${organization?.id}/application/${item.id}`}
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
