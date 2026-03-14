import { FolderSearchIcon, LoaderCircleIcon, PlusIcon } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { normalizeProfileId } from "@/lib/profile-workspace";

interface ProfilePickerProps {
  activeProfileId: string;
  profiles: string[];
  recentProfileIds: string[];
  isLoading?: boolean;
  isSwitching?: boolean;
  onSelectProfileId: (profileId: string) => void;
  onCreateProfileId: (profileId: string) => boolean | undefined;
}

export function ProfilePicker({
  activeProfileId,
  profiles,
  recentProfileIds,
  isLoading = false,
  isSwitching = false,
  onSelectProfileId,
  onCreateProfileId,
}: ProfilePickerProps) {
  const [draftProfileId, setDraftProfileId] = useState("");
  const [inputError, setInputError] = useState<string | null>(null);

  useEffect(() => {
    if (!activeProfileId) {
      return;
    }
    setDraftProfileId("");
    setInputError(null);
  }, [activeProfileId]);

  const recentProfileSet = useMemo(() => new Set(recentProfileIds), [recentProfileIds]);

  const existingProfiles = useMemo(
    () => profiles.filter((profileId) => !recentProfileSet.has(profileId)),
    [profiles, recentProfileSet],
  );

  const submitDraftProfile = () => {
    const normalizedProfileId = normalizeProfileId(draftProfileId);
    if (!normalizedProfileId) {
      setInputError("Project ID must contain letters or numbers after normalization.");
      return;
    }
    setInputError(null);
    onCreateProfileId(normalizedProfileId);
  };

  return (
    <div className="rounded-[26px] border border-sidebar-border/80 bg-sidebar-accent/45 p-4 shadow-sm">
      <div className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.28em] text-sidebar-foreground/68">
        <FolderSearchIcon className="size-3.5" />
        Project workspace
      </div>
      <div className="mt-3 space-y-4">
        <div className="space-y-2">
          <div className="flex items-center justify-between gap-3">
            <Label className="text-sidebar-foreground/76" htmlFor="profile-select">
              Existing projects
            </Label>
            <Badge
              variant="outline"
              className="rounded-full border-sidebar-border/80 bg-background/65 px-2.5 py-0.5 font-mono text-[10px] uppercase tracking-[0.16em] text-sidebar-foreground/72"
            >
              {isLoading ? "loading" : `${profiles.length} known`}
            </Badge>
          </div>
          <Select onValueChange={onSelectProfileId} value={activeProfileId}>
            <SelectTrigger
              aria-label="Existing projects"
              id="profile-select"
              size="lg"
              className="w-full border-sidebar-border bg-background/75 font-mono text-sm text-sidebar-foreground"
            >
              <SelectValue placeholder="Choose a project" />
            </SelectTrigger>
            <SelectContent size="lg" align="start">
              {recentProfileIds.length > 0 ? (
                <SelectGroup>
                  <SelectLabel>Recent</SelectLabel>
                  {recentProfileIds.map((profileId) => (
                    <SelectItem key={`recent-${profileId}`} size="lg" value={profileId}>
                      <span className="flex items-center justify-between gap-3">
                        <span>{profileId}</span>
                        {profileId === activeProfileId ? (
                          <span className="text-[10px] uppercase tracking-[0.16em] text-primary">
                            active
                          </span>
                        ) : null}
                      </span>
                    </SelectItem>
                  ))}
                </SelectGroup>
              ) : null}
              {recentProfileIds.length > 0 && existingProfiles.length > 0 ? (
                <SelectSeparator />
              ) : null}
              {existingProfiles.length > 0 ? (
                <SelectGroup>
                  <SelectLabel>All projects</SelectLabel>
                  {existingProfiles.map((profileId) => (
                    <SelectItem key={`existing-${profileId}`} size="lg" value={profileId}>
                      {profileId}
                    </SelectItem>
                  ))}
                </SelectGroup>
              ) : null}
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-2">
          <Label className="text-sidebar-foreground/76" htmlFor="profile-create-input">
            New project ID
          </Label>
          <div className="flex gap-2">
            <Input
              id="profile-create-input"
              value={draftProfileId}
              onChange={(event) => {
                setDraftProfileId(event.target.value);
                if (inputError) {
                  setInputError(null);
                }
              }}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  submitDraftProfile();
                }
              }}
              placeholder="new-project"
              className="border-sidebar-border bg-background/75 font-mono text-sm text-sidebar-foreground placeholder:text-sidebar-foreground/40"
            />
            <Button type="button" size="lg" className="shrink-0" onClick={submitDraftProfile}>
              <PlusIcon />
              Switch
            </Button>
          </div>
          <div className="flex min-h-10 items-center justify-between gap-3 text-xs leading-5 text-sidebar-foreground/65">
            <span>New IDs normalize to lowercase slug form before switching.</span>
            {isSwitching ? (
              <span className="inline-flex items-center gap-1.5 font-medium text-sidebar-foreground/80">
                <LoaderCircleIcon className="size-3.5 animate-spin" />
                Loading workspace...
              </span>
            ) : null}
          </div>
          {inputError ? <p className="text-xs text-destructive">{inputError}</p> : null}
        </div>
      </div>
    </div>
  );
}
