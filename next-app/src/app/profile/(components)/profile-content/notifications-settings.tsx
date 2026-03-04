import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Separator } from "@/components/ui/separator";

export function NotificationsSettings() {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Notification Preferences</CardTitle>
        <CardDescription>
          Choose what notifications you want to receive.
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-6">
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label className="text-base">Email Notifications</Label>

              <p className="text-muted-foreground text-sm">
                Receive notifications via email
              </p>
            </div>

            <Switch defaultChecked />
          </div>

          <Separator />

          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label className="text-base">Push Notifications</Label>

              <p className="text-muted-foreground text-sm">
                Receive push notifications in your browser
              </p>
            </div>

            <Switch />
          </div>

          <Separator />

          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label className="text-base">Marketing Emails</Label>

              <p className="text-muted-foreground text-sm">
                Receive emails about new features and updates
              </p>
            </div>

            <Switch defaultChecked />
          </div>

          <Separator />

          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label className="text-base">Weekly Summary</Label>

              <p className="text-muted-foreground text-sm">
                Get a weekly summary of your activity
              </p>
            </div>

            <Switch defaultChecked />
          </div>

          <Separator />

          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label className="text-base">Security Alerts</Label>

              <p className="text-muted-foreground text-sm">
                Important security notifications (always enabled)
              </p>
            </div>

            <Switch checked disabled />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
