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
import { useI18n } from "@/i18n";
import { GLOBAL_PROJECT_ID, isGlobalProjectId } from "@/lib/project-selection";
import { cn } from "@/lib/utils";

interface ProjectSwitcherProps {
  projectId: string;
  projects: string[];
  isLoading?: boolean;
  isCreating?: boolean;
  loadError?: string | null;
  onProjectIdChange: (value: string) => void;
  onCreateProject: (value: string) => Promise<string>;
  onRetryProjects?: () => void;
}

export function ProjectSwitcher({
  projectId,
  projects,
  isLoading = false,
  isCreating = false,
  loadError = null,
  onProjectIdChange,
  onCreateProject,
  onRetryProjects,
}: ProjectSwitcherProps) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const normalizedQuery = deferredQuery.trim().toLowerCase();
  const trimmedQuery = query.trim();
  const globalLabel = t("Global");
  const globalMatchesQuery =
    normalizedQuery.length === 0 ||
    globalLabel.toLowerCase().includes(normalizedQuery) ||
    "global".includes(normalizedQuery);
  const filteredProjects = projects.filter((candidate) =>
    candidate.toLowerCase().includes(normalizedQuery),
  );
  const exactProjectExists =
    trimmedQuery === GLOBAL_PROJECT_ID ||
    trimmedQuery.toLowerCase() === "global" ||
    trimmedQuery === globalLabel ||
    projects.some((candidate) => candidate === trimmedQuery);
  const canCreate = trimmedQuery.length > 0 && !exactProjectExists;

  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) {
      setQuery("");
    }
  };

  const handleSelect = (value: string) => {
    onProjectIdChange(value);
    handleOpenChange(false);
  };

  const handleCreate = async () => {
    if (!canCreate || isCreating) {
      return;
    }
    await onCreateProject(trimmedQuery);
    handleOpenChange(false);
  };

  const renderLabel = (value: string) => (isGlobalProjectId(value) ? globalLabel : value);

  return (
    <div className="rounded-[26px] border border-sidebar-border/80 bg-sidebar-accent/45 p-4 shadow-sm">
      <div className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.28em] text-sidebar-foreground/68">
        <FolderSearchIcon className="size-3.5" />
        {t("Current project")}
      </div>
      <div className="mt-3 space-y-2">
        <Label className="text-sidebar-foreground/76" htmlFor="project-id">
          {t("Project ID")}
        </Label>
        <Popover open={open} onOpenChange={handleOpenChange}>
          <PopoverTrigger asChild>
            <Button
              id="project-id"
              variant="outline"
              role="combobox"
              aria-expanded={open}
              className="h-auto w-full justify-between rounded-2xl border-sidebar-border bg-background/78 px-3 py-3 font-mono text-sm text-sidebar-foreground hover:bg-background"
            >
              <span className="truncate text-left">{renderLabel(projectId)}</span>
              <ChevronsUpDownIcon className="size-4 shrink-0 text-sidebar-foreground/45" />
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-[var(--radix-popover-trigger-width)] min-w-72 overflow-hidden border-sidebar-border bg-background/96 p-0 backdrop-blur-xl">
            <Command shouldFilter={false}>
              <CommandInput
                placeholder={t("Search projects or type a new ID")}
                value={query}
                onValueChange={setQuery}
              />
              <CommandList>
                {loadError ? (
                  <div className="space-y-3 px-3 py-4 text-sm">
                    <div className="rounded-2xl border border-destructive/15 bg-destructive/8 px-3 py-2.5 text-destructive">
                      {loadError}
                    </div>
                    {onRetryProjects ? (
                      <Button
                        variant="outline"
                        size="sm"
                        className="w-full justify-center"
                        onClick={onRetryProjects}
                      >
                        <RefreshCwIcon className="size-3.5" />
                        {t("Retry catalog")}
                      </Button>
                    ) : null}
                  </div>
                ) : null}
                {!loadError && isLoading && projects.length === 0 ? (
                  <div className="flex items-center justify-center gap-2 px-3 py-6 text-sm text-muted-foreground">
                    <LoaderCircleIcon className="size-4 animate-spin" />
                    {t("Loading projects...")}
                  </div>
                ) : null}
                {!loadError && globalMatchesQuery ? (
                  <CommandGroup heading={t("Contexts")}>
                    <CommandItem
                      key={GLOBAL_PROJECT_ID}
                      value={GLOBAL_PROJECT_ID}
                      onSelect={() => handleSelect(GLOBAL_PROJECT_ID)}
                    >
                      <CheckIcon
                        className={cn(
                          "size-4 text-primary transition-opacity",
                          isGlobalProjectId(projectId) ? "opacity-100" : "opacity-0",
                        )}
                      />
                      <div className="min-w-0 flex-1">
                        <div className="truncate font-medium">{globalLabel}</div>
                        <div className="truncate text-xs text-muted-foreground">
                          {t("Shared pool and allocation control across every project.")}
                        </div>
                      </div>
                      {isGlobalProjectId(projectId) ? (
                        <CommandShortcut>{t("Active")}</CommandShortcut>
                      ) : null}
                    </CommandItem>
                  </CommandGroup>
                ) : null}
                {!loadError && filteredProjects.length > 0 ? (
                  <CommandGroup heading={t("Known projects")}>
                    {filteredProjects.map((candidate) => (
                      <CommandItem
                        key={candidate}
                        value={candidate}
                        onSelect={() => handleSelect(candidate)}
                      >
                        <CheckIcon
                          className={cn(
                            "size-4 text-primary transition-opacity",
                            candidate === projectId ? "opacity-100" : "opacity-0",
                          )}
                        />
                        <div className="min-w-0 flex-1">
                          <div className="truncate font-mono text-sm">{candidate}</div>
                        </div>
                        {candidate === projectId ? (
                          <CommandShortcut>{t("Active")}</CommandShortcut>
                        ) : null}
                      </CommandItem>
                    ))}
                  </CommandGroup>
                ) : null}
                {!loadError && canCreate ? <CommandSeparator /> : null}
                {!loadError && canCreate ? (
                  <CommandGroup heading={t("Create")}>
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
                        <div className="truncate font-medium">
                          {t('Create "{value}"', { value: trimmedQuery })}
                        </div>
                        <div className="truncate text-xs text-muted-foreground">
                          {t("Start an empty project catalog entry and switch to it immediately.")}
                        </div>
                      </div>
                    </CommandItem>
                  </CommandGroup>
                ) : null}
                {!loadError && !isLoading && filteredProjects.length === 0 && !canCreate ? (
                  <CommandEmpty>
                    {t("No matching projects. Type a new ID to create one.")}
                  </CommandEmpty>
                ) : null}
              </CommandList>
            </Command>
          </PopoverContent>
        </Popover>
        <p className="text-xs leading-5 text-sidebar-foreground/60">
          {t("Search the catalog or create a new empty project before loading any feed.")}
        </p>
      </div>
    </div>
  );
}
