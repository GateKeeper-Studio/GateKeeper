import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Mail } from "lucide-react";

type ProfileHeaderProps = {
  firstName: string;
  lastName: string;
  displayName: string;
  email: string;
};

export default function ProfileHeader({
  firstName,
  lastName,
  displayName,
  email,
}: ProfileHeaderProps) {
  const initials =
    `${firstName?.charAt(0) ?? ""}${lastName?.charAt(0) ?? ""}`.toUpperCase() ||
    "?";

  return (
    <Card>
      <CardContent className="pt-4">
        <div className="flex flex-col items-start gap-6 md:flex-row md:items-center">
          <Avatar className="h-24 w-24">
            <AvatarFallback className="text-2xl">{initials}</AvatarFallback>
          </Avatar>

          <div className="flex-1 space-y-2">
            <div className="flex flex-col gap-2 md:flex-row md:items-center">
              <h1 className="text-2xl font-bold">{displayName}</h1>
              <Badge variant="secondary">Self-Service Portal</Badge>
            </div>

            <div className="text-muted-foreground flex flex-wrap gap-4 text-sm">
              <div className="flex items-center gap-1">
                <Mail className="size-4" />
                {email}
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
