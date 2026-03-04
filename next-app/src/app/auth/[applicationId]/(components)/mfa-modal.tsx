"use client";

import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Fingerprint, Mail, Smartphone } from "lucide-react";

export function MfaModal() {
  return (
    <Dialog onOpenChange={(isOpened) => isOpened}>
      <div className="text-sm text-muted-foreground text-center bg-amber-500/10 p-2 flex flex-col gap-2 rounded-md">
        <span>Cannot use this authentication method now? </span>

        <DialogTrigger className="text-foreground p-1 px-3 hover:brightness-50 transition-all rounded-sm bg-amber-600/30">
          Try another one
        </DialogTrigger>
      </div>

      <DialogContent className="top-[40%] sm:max-w-155 ">
        <DialogHeader>
          <DialogTitle>Two Factor Methods</DialogTitle>

          <DialogDescription>
            Two-factor authentication adds an additional layer of security to
            your account by requiring more than just a password to sign in.
          </DialogDescription>
        </DialogHeader>

        <Card className="w-full transition-all hover:scale-[1.01] hover:cursor-not-allowed hover:shadow-lg flex p-0 flex-row overflow-hidden">
          <div className="flex items-center justify-center p-4 bg-muted">
            <Mail className="w-6 h-6 text-muted-foreground" />
          </div>

          <CardHeader className="p-4 w-full">
            <CardTitle className="flex gap-2 justify-between">
              E-mail <Badge variant="secondary">Not Configured</Badge>
            </CardTitle>

            <CardDescription className="line-clamp-4">
              E-mail is a common two-factor authentication method that sends a
              verification code to your registered e-mail address.
            </CardDescription>
          </CardHeader>
        </Card>

        <Card className="w-full transition-all hover:scale-[1.01] hover:cursor-pointer hover:shadow-lg flex p-0 flex-row overflow-hidden">
          <div className="flex items-center justify-center p-4 bg-muted">
            <Smartphone className="w-6 h-6 text-muted-foreground" />
          </div>

          <CardHeader className="p-4 w-full">
            <CardTitle className="flex gap-2 justify-between">
              Authenticator App <Badge>Configured</Badge>
            </CardTitle>
            <CardDescription className="line-clamp-4">
              Authenticator apps generate time-based one-time codes on your
              device, providing a secure and convenient way to verify your
              identity.
            </CardDescription>
          </CardHeader>
        </Card>

        <Card className="w-full transition-all hover:scale-[1.01] hover:cursor-pointer hover:shadow-lg flex p-0 flex-row overflow-hidden">
          <div className="flex items-center justify-center p-4 bg-muted">
            <Fingerprint className="w-6 h-6 text-muted-foreground" />
          </div>

          <CardHeader className="p-4 w-full">
            <CardTitle className="flex gap-2 justify-between">
              Passkey <Badge>Configured</Badge>
            </CardTitle>
            <CardDescription className="line-clamp-4">
              Passkeys are a secure and convenient way to authenticate without
              passwords. They use your device's built-in security features to
              verify your identity.
            </CardDescription>
          </CardHeader>
        </Card>
      </DialogContent>
    </Dialog>
  );
}
