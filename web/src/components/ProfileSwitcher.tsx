import {
  CheckIcon,
  ChevronsUpDownIcon,
  FolderSearchIcon,
  LoaderCircleIcon,
  PlusIcon,
  RefreshCwIcon,
} from "lucide-react";
import { useDeferredValue, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from "@/components/ui/command";
import { Label } from "@/components/ui/label";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { cn } from "@/lib/utils";

interface ProfileSwitcherProps {
  profileId: string;
  profiles: string[];
  isLoading?: boolean;
  isCreating?: boolean;
  loadError?: string | null;
  onProfileIdChange: (value: string) => void;
  onCreateProfile: (value: string) => Promise<string>;
  onRetryProfiles?: () => void;
}

export function ProfileSwitcher({
  profileId,
  profiles,
  isLoading = false,
  isCreating = false,
  loadError = null,
  onProfileIdChange,
  onCreateProfile,
  onRetryProfiles,
}: ProfileSwitcherProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const normalizedQuery = deferredQuery.trim().toLowerCase();
  const trimmedQuery = query.trim();
  const filteredProfiles = profiles.filter((candidate) =>
    candidate.toLowerCase().includes(normalizedQuery),
  );
  const exactProfileExists = profiles.some((candidate) => candidate === trimmedQuery);
  const canCreate = trimmedQuery.length > 0 && !exactProfileExists;

  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) {
      setQuery("");
    }
  };

  const handleSelect = (value: string) => {
    onProfileIdChange(value);
    handleOpenChange(false);
  };

  const handleCreate = async () => {
    if (!canCreate || isCreating) {
      return;
    }
    await onCreateProfile(trimmedQuery);
    handleOpenChange(false);
  };

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
        <Popover open={open} onOpenChange={handleOpenChange}>
          <PopoverTrigger asChild>
            <Button
              id="profile-id"
              variant="outline"
              role="combobox"
              aria-expanded={open}
              className="h-auto w-full justify-between rounded-2xl border-sidebar-border bg-background/78 px-3 py-3 font-mono text-sm text-sidebar-foreground hover:bg-background"
            >
              <span className="truncate text-left">{profileId}</span>
              <ChevronsUpDownIcon className="size-4 shrink-0 text-sidebar-foreground/45" />
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-[var(--radix-popover-trigger-width)] min-w-72 overflow-hidden border-sidebar-border bg-background/96 p-0 backdrop-blur-xl">
            <Command shouldFilter={false}>
              <CommandInput
                placeholder="Search profiles or type a new ID"
                value={query}
                onValueChange={setQuery}
              />
              <CommandList>
                {loadError ? (
                  <div className="space-y-3 px-3 py-4 text-sm">
                    <div className="rounded-2xl border border-destructive/15 bg-destructive/8 px-3 py-2.5 text-destructive">
                      {loadError}
                    </div>
                    {onRetryProfiles ? (
                      <Button
                        variant="outline"
                        size="sm"
                        className="w-full justify-center"
                        onClick={onRetryProfiles}
                      >
                        <RefreshCwIcon className="size-3.5" />
                        Retry catalog
                      </Button>
                    ) : null}
                  </div>
                ) : null}
                {!loadError && isLoading && profiles.length === 0 ? (
                  <div className="flex items-center justify-center gap-2 px-3 py-6 text-sm text-muted-foreground">
                    <LoaderCircleIcon className="size-4 animate-spin" />
                    Loading profiles…
                  </div>
                ) : null}
                {!loadError && filteredProfiles.length > 0 ? (
                  <CommandGroup heading="Known profiles">
                    {filteredProfiles.map((candidate) => (
                      <CommandItem
                        key={candidate}
                        value={candidate}
                        onSelect={() => handleSelect(candidate)}
                      >
                        <CheckIcon
                          className={cn(
                            "size-4 text-primary transition-opacity",
                            candidate === profileId ? "opacity-100" : "opacity-0",
                          )}
                        />
                        <div className="min-w-0 flex-1">
                          <div className="truncate font-mono text-sm">{candidate}</div>
                        </div>
                        {candidate === profileId ? <CommandShortcut>Active</CommandShortcut> : null}
                      </CommandItem>
                    ))}
                  </CommandGroup>
                ) : null}
                {!loadError && canCreate ? <CommandSeparator /> : null}
                {!loadError && canCreate ? (
                  <CommandGroup heading="Create">
                    <CommandItem
                      value={`create:${trimmedQuery}`}
                      onSelect={() => void handleCreate()}
                    >
                      {isCreating ? (
                        <LoaderCircleIcon className="size-4 animate-spin text-primary" />
                      ) : (
                        <PlusIcon className="size-4 text-primary" />
                      )}
                      <div className="min-w-0 flex-1">
                        <div className="truncate font-medium">Create "{trimmedQuery}"</div>
                        <div className="truncate text-xs text-muted-foreground">
                          Start an empty profile catalog entry and switch to it immediately.
                        </div>
                      </div>
                    </CommandItem>
                  </CommandGroup>
                ) : null}
                {!loadError && !isLoading && filteredProfiles.length === 0 && !canCreate ? (
                  <CommandEmpty>No matching profiles. Type a new ID to create one.</CommandEmpty>
                ) : null}
              </CommandList>
            </Command>
          </PopoverContent>
        </Popover>
        <p className="text-xs leading-5 text-sidebar-foreground/60">
          Search the catalog or create a new empty profile before loading any feed.
        </p>
      </div>
    </div>
  );
}
