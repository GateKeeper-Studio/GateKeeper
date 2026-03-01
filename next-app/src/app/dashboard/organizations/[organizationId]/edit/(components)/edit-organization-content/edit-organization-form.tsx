"use client";

import { z } from "zod";
import { toast } from "sonner";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";

import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";

import { formSchema } from "./schema";

import { editOrganizationApi } from "@/services/settings/edit-organization";

type Props = {
  organization: {
    id: string;
    name: string;
    description: string;
  };
};

export function EditOrganizationForm({ organization }: Props) {
  const router = useRouter();
  const [isLoading, setIsLoading] = useState(false);

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      name: organization.name,
      description: organization.description,
    },
  });

  async function onSubmit(values: z.infer<typeof formSchema>) {
    setIsLoading(true);

    const [, err] = await editOrganizationApi(
      { ...values, id: organization.id },
      {
        accessToken: "fake-token",
      },
    );

    if (err) {
      console.error(err);
      toast.error("An error occurred while updating the organization.");
      setIsLoading(false);
      return;
    }

    toast.success("Organization updated successfully!");

    setIsLoading(false);

    router.push("/dashboard/organizations");
  }

  return (
    <Form {...form}>
      <form
        onSubmit={form.handleSubmit(onSubmit)}
        className="mt-4 max-w-[700px]"
      >
        <div className="grid gap-2">
          <FormField
            control={form.control}
            name="name"
            render={({ field }) => (
              <FormItem>
                <FormLabel className="flex gap-1">
                  Name
                  <span className="text-red-500">*</span>
                </FormLabel>

                <FormControl>
                  <Input
                    placeholder="Type the organization name"
                    autoComplete="name"
                    type="text"
                    {...field}
                  />
                </FormControl>

                <FormDescription></FormDescription>
                <FormMessage></FormMessage>
              </FormItem>
            )}
          />

          <FormField
            control={form.control}
            name="description"
            render={({ field }) => (
              <FormItem>
                <FormLabel className="flex gap-1">Description</FormLabel>

                <FormControl>
                  <Textarea
                    placeholder="Type the organization description"
                    autoComplete="description"
                    {...field}
                  />
                </FormControl>

                <FormDescription></FormDescription>
                <FormMessage></FormMessage>
              </FormItem>
            )}
          />

          <Button type="submit" disabled={isLoading} className="ml-auto w-fit">
            {isLoading ? "Editing Organization..." : "Edit Organization"}
          </Button>
        </div>
      </form>
    </Form>
  );
}
