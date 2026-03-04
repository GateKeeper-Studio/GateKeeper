"use client";

import { useState, useCallback, useEffect } from "react";
import { ArrowLeft, AlertTriangle } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

import ProfileHeader from "./profile-header";
import ProfileContent from "./profile-content";
import { StepUpProvider } from "./step-up-context";
import { SessionProvider, useSession } from "./session-context";
import { ReauthDialog } from "./profile-content/dialogs/reauth-dialog";

import { api } from "@/services/base/gatekeeper-api";
import type { MfaMethodItem } from "@/services/account/list-mfa-methods";

type MeResponse = {
  firstName: string;
  lastName: string;
  displayName: string;
  phoneNumber: string | null;
  address: string | null;
  hasMfa: boolean;
  mfaMethod: string | null;
  mfaMethods: MfaMethodItem[];
};

type ProfileWrapperProps = {
  accessToken: string;
  userId: string;
  applicationId: string;
  email: string;
  firstName: string;
  lastName: string;
  displayName: string;
  returnUrl: string | null;
};

export default function ProfileWrapper(props: ProfileWrapperProps) {
  return (
    <SessionProvider initialAccessToken={props.accessToken}>
      <ProfileWrapperInner {...props} />
    </SessionProvider>
  );
}

function ProfileWrapperInner({
  userId,
  applicationId,
  email,
  firstName,
  lastName,
  displayName,
  returnUrl,
}: ProfileWrapperProps) {
  const { accessToken, sessionExpired } = useSession();

  const [currentFirstName, setCurrentFirstName] = useState(firstName);
  const [currentLastName, setCurrentLastName] = useState(lastName);
  const [currentDisplayName, setCurrentDisplayName] = useState(displayName);
  const [phoneNumber, setPhoneNumber] = useState<string | null>(null);
  const [address, setAddress] = useState<string | null>(null);
  const [hasMfa, setHasMfa] = useState(false);
  const [mfaMethods, setMfaMethods] = useState<MfaMethodItem[]>([]);
  const [preferredMethod, setPreferredMethod] = useState<string | null>(null);
  const [reauthOpen, setReauthOpen] = useState(false);
  const [reauthResolve, setReauthResolve] = useState<
    ((token: string | null) => void) | null
  >(null);

  // Fetch MFA status from the server
  const fetchMfaStatus = useCallback(() => {
    api
      .get<MeResponse>(`/v1/account/me`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      .then(({ data }) => {
        setCurrentFirstName(data.firstName);
        setCurrentLastName(data.lastName);
        setCurrentDisplayName(data.displayName);
        setPhoneNumber(data.phoneNumber);
        setAddress(data.address);
        setHasMfa(data.hasMfa);
        setMfaMethods(data.mfaMethods ?? []);
        setPreferredMethod(data.mfaMethod ?? null);
      })
      .catch(() => {
        // 401 is handled by SessionProvider (auto-refresh)
      });
  }, [accessToken]);

  useEffect(() => {
    fetchMfaStatus();
  }, [fetchMfaStatus]);

  const onRequestReauth = useCallback((): Promise<string | null> => {
    return new Promise((resolve) => {
      setReauthResolve(() => resolve);
      setReauthOpen(true);
    });
  }, []);

  const handleReauthSuccess = useCallback(
    (token: string) => {
      reauthResolve?.(token);
      setReauthResolve(null);
    },
    [reauthResolve],
  );

  const handleReauthClose = useCallback(
    (open: boolean) => {
      setReauthOpen(open);
      if (!open) {
        reauthResolve?.(null);
        setReauthResolve(null);
      }
    },
    [reauthResolve],
  );

  return (
    <StepUpProvider onRequestReauth={onRequestReauth}>
      <main className="flex flex-col p-4">
        <div className="mx-auto w-full max-w-4xl space-y-6 px-4">
          {returnUrl && (
            <Button variant="ghost" size="sm" asChild>
              <a href={returnUrl}>
                <ArrowLeft className="mr-2 size-4" />
                Back to application
              </a>
            </Button>
          )}

          {sessionExpired && (
            <Alert variant="destructive">
              <AlertTriangle className="size-4" />
              <AlertTitle>Session Expired</AlertTitle>
              <AlertDescription className="flex items-center justify-between">
                <span>
                  Your access token has expired. Please go back and sign in
                  again.
                </span>

                {returnUrl && (
                  <Button variant="outline" size="sm" asChild>
                    <a href={returnUrl}>Return to application</a>
                  </Button>
                )}
              </AlertDescription>
            </Alert>
          )}

          <ProfileHeader
            firstName={currentFirstName}
            lastName={currentLastName}
            displayName={currentDisplayName}
            email={email}
          />

          <ProfileContent
            applicationId={applicationId}
            userId={userId}
            email={email}
            firstName={currentFirstName}
            lastName={currentLastName}
            displayName={currentDisplayName}
            phoneNumber={phoneNumber}
            address={address}
            hasMfa={hasMfa}
            mfaMethods={mfaMethods}
            preferredMethod={preferredMethod}
            onMfaChange={fetchMfaStatus}
            onProfileChange={fetchMfaStatus}
          />
        </div>
      </main>

      <ReauthDialog
        open={reauthOpen}
        onOpenChange={handleReauthClose}
        onSuccess={handleReauthSuccess}
        applicationId={applicationId}
        hasMfa={hasMfa}
      />
    </StepUpProvider>
  );
}
