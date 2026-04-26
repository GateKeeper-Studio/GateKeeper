"use client";

import { Checkbox } from "@/components/ui/checkbox";

import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { FormType } from "./user-detail-form";

import {
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
} from "@/components/ui/form";
import { useParams } from "next/navigation";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

import { APIError } from "@/types/service-options";
import { useApplicationRolesSWR } from "@/services/dashboard/use-application-roles-swr";

type Props = {
  form: FormType;
  applicationId?: string;
};

export function ApplicationRolesSection({
  form,
  applicationId: applicationIdProp,
}: Props) {
  const { tenantId, applicationId: applicationIdParam } = useParams() as {
    tenantId: string;
    applicationId?: string;
  };

  const applicationId = applicationIdProp || applicationIdParam || "";

  const { data, error, isLoading } = useApplicationRolesSWR(
    { applicationId, tenantId, pageSize: 100 },
    { accessToken: "" },
  );

  const errorData = error as APIError;

  return (
    <div className="flex flex-col gap-1">
      <span className="text-sm font-medium"> Application Roles </span>

      <span className="text-muted-foreground my-2 text-sm">
        Choose the roles that will be assigned to this user.
      </span>

      {isLoading && (
        <div className="flex flex-col gap-2">
          <Skeleton className="w-[7rem] h-[1.25rem] rounded-sm" />
          <Skeleton className="w-[10rem] h-[1.25rem] rounded-sm" />
        </div>
      )}

      {error && (
        <Alert>
          <AlertTitle>Error on trying to list roles</AlertTitle>
          <AlertDescription>
            {errorData?.response?.data?.message ||
              "Error on fetching roles data."}
          </AlertDescription>
        </Alert>
      )}

      {data?.data?.map((role) => (
        <FormField
          key={role.id}
          control={form.control}
          name="roles"
          render={({ field }) => (
            <FormItem className="flex items-center space-x-2">
              <FormControl>
                <Checkbox
                  checked={field.value.includes(role.id)}
                  onCheckedChange={(checked) => {
                    if (checked) {
                      field.onChange([...field.value, role.id]);
                    } else {
                      field.onChange(field.value.filter((r) => r !== role.id));
                    }
                  }}
                  aria-labelledby={role.id}
                  id={role.id}
                />
              </FormControl>

              <Tooltip>
                <TooltipTrigger type="button">
                  <FormLabel
                    htmlFor={role.id}
                    className="text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                  >
                    {role.name}
                  </FormLabel>
                </TooltipTrigger>

                <TooltipContent>{role.description}</TooltipContent>
              </Tooltip>

              <FormDescription></FormDescription>
            </FormItem>
          )}
        />
      ))}
    </div>
  );
}
