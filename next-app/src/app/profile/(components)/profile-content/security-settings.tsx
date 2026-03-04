"use client";

import { useState, useCallback, useEffect } from "react";
import {
  Key,
  Shield,
  Smartphone,
  ShieldAlert,
  Loader2,
  Globe,
  Mail,
  Fingerprint,
  Star,
} from "lucide-react";
import { toast } from "sonner";

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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

import { useStepUp } from "../step-up-context";
import { useSession } from "../session-context";
import { ReauthDialog } from "./dialogs/reauth-dialog";
import { ChangePasswordDialog } from "./dialogs/change-password-dialog";
import { EnableMfaDialog } from "./dialogs/enable-mfa-dialog";
import { BackupCodesDialog } from "./dialogs/backup-codes-dialog";
import { SessionsDialog } from "./dialogs/sessions-dialog";

import {
  listMfaMethodsApi,
  type MfaMethodItem,
} from "@/services/account/list-mfa-methods";
import { updatePreferredMfaApi } from "@/services/account/update-preferred-mfa";
import { disableMfaMethodApi } from "@/services/account/disable-mfa-method";

type SecuritySettingsProps = {
  applicationId: string;
  userId: string;
  hasMfa: boolean;
  mfaMethods: MfaMethodItem[];
  preferredMethod: string | null;
  onMfaChange: () => void;
};

const METHOD_LABELS: Record<string, string> = {
  totp: "Authenticator App",
  email: "Email",
  webauthn: "Passkey / Security Key",
};

const METHOD_ICONS: Record<string, typeof Smartphone> = {
  totp: Smartphone,
  email: Mail,
  webauthn: Fingerprint,
};

export function SecuritySettings({
  applicationId,
  userId,
  hasMfa,
  mfaMethods,
  preferredMethod,
  onMfaChange,
}: SecuritySettingsProps) {
  const { accessToken } = useSession();
  const { stepUpToken, requestStepUp } = useStepUp();

  // Dialog visibility
  const [showReauth, setShowReauth] = useState(false);
  const [showChangePassword, setShowChangePassword] = useState(false);
  const [showEnableMfa, setShowEnableMfa] = useState(false);
  const [showBackupCodes, setShowBackupCodes] = useState(false);
  const [showSessions, setShowSessions] = useState(false);

  // Pending action after reauth
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const [pendingToken, setPendingToken] = useState<string | null>(null);

  const [disablingMethod, setDisablingMethod] = useState<string | null>(null);
  const [updatingPreferred, setUpdatingPreferred] = useState(false);

  // Which MFA type to configure via dialog
  const [configuringMfaType, setConfiguringMfaType] = useState<string>("totp");

  const enabledMethods = mfaMethods.filter((m) => m.enabled);

  const handleReauthSuccess = useCallback(
    (token: string) => {
      setPendingToken(token);

      if (pendingAction === "change-password") {
        setShowChangePassword(true);
      } else if (pendingAction?.startsWith("disable-mfa:")) {
        const method = pendingAction.split(":")[1];
        handleDisableMfaMethod(method, token);
      } else if (pendingAction === "backup-codes") {
        setShowBackupCodes(true);
      } else if (pendingAction === "sessions") {
        setShowSessions(true);
      }

      setPendingAction(null);
    },
    [pendingAction],
  );

  const requireStepUp = useCallback(
    (action: string) => {
      if (stepUpToken) {
        setPendingToken(stepUpToken);
        if (action === "change-password") setShowChangePassword(true);
        else if (action === "backup-codes") setShowBackupCodes(true);
        else if (action === "sessions") setShowSessions(true);
        else if (action.startsWith("disable-mfa:")) {
          const method = action.split(":")[1];
          handleDisableMfaMethod(method, stepUpToken);
        }
      } else {
        setPendingAction(action);
        setShowReauth(true);
      }
    },
    [stepUpToken],
  );

  const handleDisableMfaMethod = useCallback(
    async (method: string, token: string) => {
      setDisablingMethod(method);

      const [, err] = await disableMfaMethodApi(
        { method, stepUpToken: token },
        { accessToken },
      );

      setDisablingMethod(null);

      if (err) {
        if (err.response?.status === 401) {
          toast.error(
            "Your session has expired. Please go back and sign in again.",
          );
          return;
        }
        toast.error(
          err.response?.data?.message ||
            `Failed to disable ${METHOD_LABELS[method] || method}`,
        );
        return;
      }

      toast.success(`${METHOD_LABELS[method] || method} has been disabled`);
      onMfaChange();
    },
    [accessToken, onMfaChange],
  );

  const handlePreferredChange = useCallback(
    async (method: string) => {
      setUpdatingPreferred(true);

      const [, err] = await updatePreferredMfaApi(
        { preferredMethod: method },
        { accessToken },
      );

      setUpdatingPreferred(false);

      if (err) {
        toast.error(
          err.response?.data?.message || "Failed to update preferred method",
        );
        return;
      }

      toast.success(
        `Preferred MFA method set to ${METHOD_LABELS[method] || method}`,
      );
      onMfaChange();
    },
    [accessToken, onMfaChange],
  );

  const handleEnableMfaSuccess = useCallback(() => {
    onMfaChange();
  }, [onMfaChange]);

  const isMethodEnabled = (type: string) =>
    mfaMethods.some((m) => m.type === type && m.enabled);

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Security Settings</CardTitle>
          <CardDescription>
            Manage your account security and authentication.
          </CardDescription>
        </CardHeader>

        <CardContent className="space-y-6">
          <div className="space-y-4">
            {/* Change Password */}
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <Label className="text-base">Password</Label>

                <p className="text-muted-foreground text-sm">
                  Keep your account secure with a strong password
                </p>
              </div>

              <Button
                variant="outline"
                onClick={() => requireStepUp("change-password")}
              >
                <Key className="mr-2 h-4 w-4" />
                Change Password
              </Button>
            </div>

            <Separator />

            {/* Two-Factor Authentication Overview */}
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <Label className="text-base">Two-Factor Authentication</Label>

                  <p className="text-muted-foreground text-sm">
                    Configure multiple MFA methods for your account
                  </p>
                </div>

                <Badge
                  variant="outline"
                  className={
                    hasMfa
                      ? "border-green-200 bg-green-50 text-green-700 dark:border-green-800 dark:bg-green-950 dark:text-green-400"
                      : "border-orange-200 bg-orange-50 text-orange-700 dark:border-orange-800 dark:bg-orange-950 dark:text-orange-400"
                  }
                >
                  {hasMfa
                    ? `${enabledMethods.length} method${enabledMethods.length !== 1 ? "s" : ""} enabled`
                    : "Not configured"}
                </Badge>
              </div>

              {/* Preferred Method Selector */}
              {enabledMethods.length > 0 && (
                <div className="flex items-center justify-between rounded-lg border p-3">
                  <div className="flex items-center gap-2">
                    <Star className="size-4 text-amber-500" />
                    <Label className="text-sm font-medium">
                      Preferred Method
                    </Label>
                  </div>

                  <Select
                    value={preferredMethod ?? ""}
                    onValueChange={handlePreferredChange}
                    disabled={updatingPreferred}
                  >
                    <SelectTrigger className="w-50">
                      <SelectValue placeholder="Select method" />
                    </SelectTrigger>

                    <SelectContent>
                      {enabledMethods.map((m) => {
                        const Icon = METHOD_ICONS[m.type] || Shield;

                        return (
                          <SelectItem key={m.type} value={m.type}>
                            <div className="flex items-center gap-2">
                              <Icon className="size-4" />
                              {METHOD_LABELS[m.type] || m.type}
                            </div>
                          </SelectItem>
                        );
                      })}
                    </SelectContent>
                  </Select>
                </div>
              )}

              {/* MFA Method Cards */}
              <div className="space-y-3">
                {/* Authenticator App */}
                <MfaMethodRow
                  type="totp"
                  enabled={isMethodEnabled("totp")}
                  isPreferred={preferredMethod === "totp"}
                  isDisabling={disablingMethod === "totp"}
                  onEnable={() => {
                    setConfiguringMfaType("totp");
                    setShowEnableMfa(true);
                  }}
                  onDisable={() => requireStepUp("disable-mfa:totp")}
                />

                {/* Email MFA */}
                <MfaMethodRow
                  type="email"
                  enabled={isMethodEnabled("email")}
                  isPreferred={preferredMethod === "email"}
                  isDisabling={disablingMethod === "email"}
                  onEnable={() => {
                    setConfiguringMfaType("email");
                    setShowEnableMfa(true);
                  }}
                  onDisable={() => requireStepUp("disable-mfa:email")}
                />

                {/* WebAuthn / Passkey */}
                <MfaMethodRow
                  type="webauthn"
                  enabled={isMethodEnabled("webauthn")}
                  isPreferred={preferredMethod === "webauthn"}
                  isDisabling={disablingMethod === "webauthn"}
                  onEnable={() => {
                    setConfiguringMfaType("webauthn");
                    setShowEnableMfa(true);
                  }}
                  onDisable={() => requireStepUp("disable-mfa:webauthn")}
                />
              </div>
            </div>

            <Separator />

            {/* Backup Codes */}
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <Label className="text-base">Backup Recovery Codes</Label>

                <p className="text-muted-foreground text-sm">
                  Generate codes to use if you lose your authenticator
                </p>
              </div>

              <Button
                variant="outline"
                onClick={() => requireStepUp("backup-codes")}
                disabled={!hasMfa}
              >
                <ShieldAlert className="mr-2 h-4 w-4" />
                Generate Codes
              </Button>
            </div>

            <Separator />

            {/* Active Sessions */}
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <Label className="text-base">Active Sessions</Label>

                <p className="text-muted-foreground text-sm">
                  Manage devices that are logged into your account
                </p>
              </div>

              <Button
                variant="outline"
                onClick={() => requireStepUp("sessions")}
              >
                <Globe className="mr-2 h-4 w-4" />
                View Sessions
              </Button>
            </div>
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
        <ChangePasswordDialog
          open={showChangePassword}
          onOpenChange={setShowChangePassword}
          applicationId={applicationId}
          stepUpToken={pendingToken}
        />
      )}

      <EnableMfaDialog
        open={showEnableMfa}
        onOpenChange={setShowEnableMfa}
        onSuccess={handleEnableMfaSuccess}
        applicationId={applicationId}
        userId={userId}
        mfaType={configuringMfaType}
      />

      {pendingToken && (
        <BackupCodesDialog
          open={showBackupCodes}
          onOpenChange={setShowBackupCodes}
          applicationId={applicationId}
          stepUpToken={pendingToken}
        />
      )}

      <SessionsDialog
        open={showSessions}
        onOpenChange={setShowSessions}
        applicationId={applicationId}
        stepUpToken={pendingToken ?? stepUpToken}
        onRequestStepUp={requestStepUp}
      />
    </>
  );
}

// --- Sub-component for each MFA method row ---

type MfaMethodRowProps = {
  type: string;
  enabled: boolean;
  isPreferred: boolean;
  isDisabling: boolean;
  onEnable: () => void;
  onDisable: () => void;
};

function MfaMethodRow({
  type,
  enabled,
  isPreferred,
  isDisabling,
  onEnable,
  onDisable,
}: MfaMethodRowProps) {
  const Icon = METHOD_ICONS[type] || Shield;
  const label = METHOD_LABELS[type] || type;

  const descriptions: Record<string, string> = {
    totp: "Use an authenticator app like Google Authenticator or Authy",
    email: "Receive a verification code via email",
    webauthn: "Use a passkey, fingerprint, or security key",
  };

  return (
    <div className="flex items-center justify-between rounded-lg border p-3">
      <div className="flex items-center gap-3">
        <div className="flex size-9 items-center justify-center rounded-md bg-muted">
          <Icon className="size-5" />
        </div>

        <div className="space-y-0.5">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">{label}</span>

            {isPreferred && (
              <Badge variant="secondary" className="text-xs px-1.5 py-0 h-5">
                Preferred
              </Badge>
            )}
          </div>

          <p className="text-xs text-muted-foreground">
            {descriptions[type] || ""}
          </p>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <Badge
          variant="outline"
          className={
            enabled
              ? "border-green-200 bg-green-50 text-green-700 dark:border-green-800 dark:bg-green-950 dark:text-green-400"
              : "border-muted text-muted-foreground"
          }
        >
          {enabled ? "Enabled" : "Disabled"}
        </Badge>

        {enabled ? (
          <Button
            variant="outline"
            size="sm"
            onClick={onDisable}
            disabled={isDisabling}
          >
            {isDisabling ? (
              <Loader2 className="mr-2 size-4 animate-spin" />
            ) : (
              <Shield className="mr-2 h-4 w-4" />
            )}
            Disable
          </Button>
        ) : (
          <Button variant="outline" size="sm" onClick={onEnable}>
            <Shield className="mr-2 h-4 w-4" />
            Enable
          </Button>
        )}
      </div>
    </div>
  );
}
