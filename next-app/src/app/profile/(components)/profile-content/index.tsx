"use client";

import { useState } from "react";
import { Shield, UserRound, SquareUserRound, Bell } from "lucide-react";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

import { PersonalSettings } from "./personal-settings";
import { AccountSettings } from "./account-settings";
import { SecuritySettings } from "./security-settings";
import { NotificationsSettings } from "./notifications-settings";

import type { MfaMethodItem } from "@/services/account/list-mfa-methods";

type ProfileContentProps = {
  applicationId: string;
  userId: string;
  email: string;
  firstName: string;
  lastName: string;
  displayName: string;
  phoneNumber: string | null;
  address: string | null;
  hasMfa: boolean;
  mfaMethods: MfaMethodItem[];
  preferredMethod: string | null;
  onMfaChange: () => void;
  onProfileChange: () => void;
};

export default function ProfileContent({
  applicationId,
  userId,
  email,
  firstName,
  lastName,
  displayName,
  phoneNumber,
  address,
  hasMfa,
  mfaMethods,
  preferredMethod,
  onMfaChange,
  onProfileChange,
}: ProfileContentProps) {
  return (
    <Tabs defaultValue="personal" className="space-y-6">
      <TabsList className="grid w-full grid-cols-4">
        <TabsTrigger value="personal">
          <UserRound /> Personal
        </TabsTrigger>

        <TabsTrigger value="account">
          <SquareUserRound /> Account
        </TabsTrigger>

        <TabsTrigger value="security">
          <Shield /> Security
        </TabsTrigger>

        <TabsTrigger value="notifications">
          <Bell />
          Notifications
        </TabsTrigger>
      </TabsList>

      {/* Personal Information */}
      <TabsContent value="personal" className="space-y-6">
        <PersonalSettings
          firstName={firstName}
          lastName={lastName}
          displayName={displayName}
          phoneNumber={phoneNumber}
          address={address}
          onProfileChange={onProfileChange}
        />
      </TabsContent>

      {/* Account Settings */}
      <TabsContent value="account" className="space-y-6">
        <AccountSettings
          applicationId={applicationId}
          email={email}
          hasMfa={hasMfa}
        />
      </TabsContent>

      {/* Security Settings */}
      <TabsContent value="security" className="space-y-6">
        <SecuritySettings
          applicationId={applicationId}
          userId={userId}
          hasMfa={hasMfa}
          mfaMethods={mfaMethods}
          preferredMethod={preferredMethod}
          onMfaChange={onMfaChange}
        />
      </TabsContent>

      {/* Notification Settings */}
      <TabsContent value="notifications" className="space-y-6">
        <NotificationsSettings />
      </TabsContent>
    </Tabs>
  );
}
