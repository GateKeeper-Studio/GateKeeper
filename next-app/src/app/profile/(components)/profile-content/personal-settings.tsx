"use client";

import { useState } from "react";
import { toast } from "sonner";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Edit } from "lucide-react";
import { Separator } from "@/components/ui/separator";
import { LoadingSpinner } from "@/components/ui/loading-spinner";

import { useSession } from "../session-context";
import { updateProfileApi } from "@/services/account/update-profile";

type PersonalSettingsProps = {
  firstName: string;
  lastName: string;
  displayName: string;
  phoneNumber: string | null;
  address: string | null;
  onProfileChange: () => void;
};

export function PersonalSettings({
  firstName,
  lastName,
  displayName,
  phoneNumber,
  address,
  onProfileChange,
}: PersonalSettingsProps) {
  const { accessToken } = useSession();

  const [isEditEnabled, setIsEditEnabled] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const [editFirstName, setEditFirstName] = useState(firstName);
  const [editLastName, setEditLastName] = useState(lastName);
  const [editDisplayName, setEditDisplayName] = useState(displayName);
  const [editPhoneNumber, setEditPhoneNumber] = useState(phoneNumber ?? "");
  const [editAddress, setEditAddress] = useState(address ?? "");

  function handleEnableEdit() {
    setEditFirstName(firstName);
    setEditLastName(lastName);
    setEditDisplayName(displayName);
    setEditPhoneNumber(phoneNumber ?? "");
    setEditAddress(address ?? "");
    setIsEditEnabled(true);
  }

  function handleCancel() {
    setIsEditEnabled(false);
  }

  async function handleSave() {
    setIsLoading(true);

    const [, err] = await updateProfileApi(
      {
        firstName: editFirstName.trim(),
        lastName: editLastName.trim(),
        displayName: editDisplayName.trim(),
        phoneNumber: editPhoneNumber.trim() || null,
        address: editAddress.trim() || null,
      },
      { accessToken },
    );

    setIsLoading(false);

    if (err) {
      toast.error(err.response?.data?.message || "Failed to update profile");
      return;
    }

    toast.success("Profile updated successfully");
    setIsEditEnabled(false);
    onProfileChange();
  }

  return (
    <Card>
      <CardHeader className="relative">
        <CardTitle>Personal Information</CardTitle>

        <CardDescription>
          Update your personal details and profile information.
        </CardDescription>

        <Button
          onClick={isEditEnabled ? handleCancel : handleEnableEdit}
          variant={isEditEnabled ? "outline" : "default"}
          className="w-fit absolute right-4 top-4"
        >
          <Edit />
          {isEditEnabled ? "Disable Edit" : "Enable Edit"}
        </Button>
      </CardHeader>

      <CardContent className="space-y-6">
        <div className="grid grid-cols-1 gap-6 md:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="displayName">Display Name</Label>
            <Input
              id="displayName"
              value={isEditEnabled ? editDisplayName : displayName}
              onChange={(e) => setEditDisplayName(e.target.value)}
              disabled={!isEditEnabled}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="firstName">First Name</Label>
            <Input
              id="firstName"
              value={isEditEnabled ? editFirstName : firstName}
              onChange={(e) => setEditFirstName(e.target.value)}
              disabled={!isEditEnabled}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="lastName">Last Name</Label>
            <Input
              id="lastName"
              value={isEditEnabled ? editLastName : lastName}
              onChange={(e) => setEditLastName(e.target.value)}
              disabled={!isEditEnabled}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="phone">Phone</Label>
            <Input
              id="phone"
              value={isEditEnabled ? editPhoneNumber : (phoneNumber ?? "")}
              onChange={(e) => setEditPhoneNumber(e.target.value)}
              disabled={!isEditEnabled}
            />
          </div>
        </div>

        <div className="space-y-2">
          <Label htmlFor="address">Address</Label>
          <Input
            id="address"
            value={isEditEnabled ? editAddress : (address ?? "")}
            onChange={(e) => setEditAddress(e.target.value)}
            disabled={!isEditEnabled}
          />
        </div>
      </CardContent>

      {isEditEnabled && (
        <>
          <Separator className="mt-4" />
          <div className="w-full flex gap-4 p-4 justify-end">
            <Button
              variant="outline"
              onClick={handleCancel}
              disabled={isLoading}
            >
              Cancel
            </Button>
            <Button
              onClick={handleSave}
              disabled={isLoading}
              className="relative"
            >
              {isLoading && <LoadingSpinner className="absolute left-4" />}
              Save Changes
            </Button>
          </div>
        </>
      )}
    </Card>
  );
}
