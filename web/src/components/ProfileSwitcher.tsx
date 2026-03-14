import { FolderSearchIcon } from "lucide-react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface ProfileSwitcherProps {
  profileId: string;
  onProfileIdChange: (value: string) => void;
}

export function ProfileSwitcher({ profileId, onProfileIdChange }: ProfileSwitcherProps) {
  return (
    <div className="rounded-[26px] border border-sidebar-border/80 bg-sidebar-accent/45 p-4 shadow-sm">
      <div className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.28em] text-sidebar-foreground/68">
        <FolderSearchIcon className="size-3.5" />
        Active profile
      </div>
      <div className="mt-3 space-y-2">
        <Label className="text-sidebar-foreground/76" htmlFor="profile-id">
          Profile ID
        </Label>
        <Input
          id="profile-id"
          value={profileId}
          onChange={(event) => onProfileIdChange(event.target.value)}
          placeholder="default"
          className="border-sidebar-border bg-background/75 font-mono text-sm text-sidebar-foreground placeholder:text-sidebar-foreground/40"
        />
      </div>
    </div>
  );
}
