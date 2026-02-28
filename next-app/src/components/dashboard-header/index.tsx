import { Breadcrumbs } from "./bread-crumbs";
import { SearchCommand } from "./search-command";

type Props = {
  breadcrumbs: {
    items: {
      name: string;
      path?: string;
    }[];
    disableSideBar?: boolean;
  };
};

export function DashboardHeader({ breadcrumbs }: Props) {
  return (
    <header className="flex items-center w-full py-1 px-2 border-b border-gray-200 dark:border-gray-700 gap-4">
      <Breadcrumbs {...breadcrumbs} />
      <SearchCommand />
    </header>
  );
}
