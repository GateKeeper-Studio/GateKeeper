import { Skeleton } from "@/components/ui/skeleton";
import { Background } from "../(components)/background";

export default function LoadingMfaWebAuthnPage() {
  return (
    <Background application={null} page="sign-in">
      <div className="flex flex-col space-y-2 text-center">
        <h1 className="text-2xl font-semibold tracking-tight">
          Passkey Authentication
        </h1>
        <p className="text-muted-foreground text-sm">
          Use your passkey or security key to verify your identity
        </p>
      </div>

      <div className="flex flex-col gap-3 items-center">
        <Skeleton className="w-full h-[2.5rem]" />
        <Skeleton className="w-full h-[2.5rem]" />
      </div>
    </Background>
  );
}
