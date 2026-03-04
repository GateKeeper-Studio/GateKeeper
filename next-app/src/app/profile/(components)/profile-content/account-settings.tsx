"use client";

import { useState, useCallback } from "react";
import { Mail, Trash2 } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";

import { useStepUp } from "../step-up-context";
import { ReauthDialog } from "./dialogs/reauth-dialog";
import { ChangeEmailDialog } from "./dialogs/change-email-dialog";

type AccountSettingsProps = {
  applicationId: string;
  email: string;
  hasMfa: boolean;
};

export function AccountSettings({
  applicationId,
  email,
  hasMfa,
}: AccountSettingsProps) {
  const { stepUpToken } = useStepUp();

  const [showReauth, setShowReauth] = useState(false);
  const [showChangeEmail, setShowChangeEmail] = useState(false);
  const [pendingToken, setPendingToken] = useState<string | null>(null);

  const handleReauthSuccess = useCallback((token: string) => {
    setPendingToken(token);
    setShowChangeEmail(true);
  }, []);

  const handleChangeEmail = () => {
    if (stepUpToken) {
      setPendingToken(stepUpToken);
      setShowChangeEmail(true);
    } else {
      setShowReauth(true);
    }
  };

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Account Settings</CardTitle>
          <CardDescription>
            Manage your account information and preferences.
          </CardDescription>
        </CardHeader>

        <CardContent className="space-y-6">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label className="text-base">Account Status</Label>

              <p className="text-muted-foreground text-sm">
                Your account is currently active
              </p>
            </div>

            <Badge
              variant="outline"
              className="border-green-200 bg-green-50 text-green-700 dark:border-green-800 dark:bg-green-950 dark:text-green-400"
            >
              Active
            </Badge>
          </div>

          <Separator />

          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label className="text-base">Email Address</Label>

              <p className="text-muted-foreground text-sm">{email}</p>
            </div>

            <Button variant="outline" onClick={handleChangeEmail}>
              <Mail className="mr-2 h-4 w-4" />
              Change Email
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card className="border-destructive/50">
        <CardHeader>
          <CardTitle className="text-destructive">Danger Zone</CardTitle>

          <CardDescription>
            Irreversible and destructive actions
          </CardDescription>
        </CardHeader>

        <CardContent>
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label className="text-base">Delete Account</Label>

              <p className="text-muted-foreground text-sm">
                Permanently delete your account and all data
              </p>
            </div>

            <Button variant="destructive" disabled>
              <Trash2 className="mr-2 h-4 w-4" />
              Delete Account
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Dialogs */}
      <ReauthDialog
        open={showReauth}
        onOpenChange={setShowReauth}
        onSuccess={handleReauthSuccess}
        applicationId={applicationId}
        hasMfa={hasMfa}
      />

      {pendingToken && (
        <ChangeEmailDialog
          open={showChangeEmail}
          onOpenChange={setShowChangeEmail}
          applicationId={applicationId}
          stepUpToken={pendingToken}
          currentEmail={email}
        />
      )}
    </>
  );
}
