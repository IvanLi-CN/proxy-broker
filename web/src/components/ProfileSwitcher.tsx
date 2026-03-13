import { FolderSearchIcon } from "lucide-react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface ProfileSwitcherProps {
  profileId: string;
  onProfileIdChange: (value: string) => void;
}

export function ProfileSwitcher({ profileId, onProfileIdChange }: ProfileSwitcherProps) {
  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.24em] text-sidebar-foreground/70">
        <FolderSearchIcon className="size-3.5" />
        Active profile
      </div>
      <div className="space-y-1.5">
        <Label className="text-sidebar-foreground/80" htmlFor="profile-id">
          Profile ID
        </Label>
        <Input
          id="profile-id"
          value={profileId}
          onChange={(event) => onProfileIdChange(event.target.value)}
          placeholder="default"
          className="border-sidebar-border bg-sidebar-accent/30 text-sidebar-foreground placeholder:text-sidebar-foreground/45"
        />
      </div>
    </div>
  );
}
