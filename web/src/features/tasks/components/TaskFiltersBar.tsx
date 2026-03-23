import { FilterIcon } from "lucide-react";

import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { formatTaskKind, formatTaskStatus, formatTaskTrigger } from "@/lib/tasks-view";
import type { TaskRunKind, TaskRunStatus, TaskRunTrigger } from "@/lib/types";

interface TaskFiltersBarProps {
  scope: "current" | "all";
  kind?: TaskRunKind;
  status?: TaskRunStatus;
  trigger?: TaskRunTrigger;
  runningOnly: boolean;
  onScopeChange: (value: "current" | "all") => void;
  onKindChange: (value?: TaskRunKind) => void;
  onStatusChange: (value?: TaskRunStatus) => void;
  onTriggerChange: (value?: TaskRunTrigger) => void;
  onRunningOnlyChange: (value: boolean) => void;
}

const taskKinds: TaskRunKind[] = [
  "subscription_sync",
  "metadata_refresh_incremental",
  "metadata_refresh_full",
];
const taskStatuses: TaskRunStatus[] = ["queued", "running", "succeeded", "failed", "skipped"];
const taskTriggers: TaskRunTrigger[] = ["schedule", "post_load"];

export function TaskFiltersBar({
  scope,
  kind,
  status,
  trigger,
  runningOnly,
  onScopeChange,
  onKindChange,
  onStatusChange,
  onTriggerChange,
  onRunningOnlyChange,
}: TaskFiltersBarProps) {
  return (
    <div className="rounded-[28px] border border-border/70 bg-card/95 p-4 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
      <div className="flex flex-wrap items-center gap-3">
        <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
          <FilterIcon className="size-4 text-primary" />
          Task filters
        </div>
        <div className="flex flex-1 flex-wrap items-center gap-3">
          <Select
            value={scope}
            onValueChange={(value) => onScopeChange(value as "current" | "all")}
          >
            <SelectTrigger className="min-w-[150px] rounded-full bg-background/80">
              <SelectValue placeholder="View scope" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="current">Current profile</SelectItem>
              <SelectItem value="all">All profiles</SelectItem>
            </SelectContent>
          </Select>

          <Select
            value={kind ?? "all"}
            onValueChange={(value) =>
              onKindChange(value === "all" ? undefined : (value as TaskRunKind))
            }
          >
            <SelectTrigger className="min-w-[180px] rounded-full bg-background/80">
              <SelectValue placeholder="Task kind" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All kinds</SelectItem>
              {taskKinds.map((item) => (
                <SelectItem key={item} value={item}>
                  {formatTaskKind(item)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Select
            value={status ?? "all"}
            onValueChange={(value) =>
              onStatusChange(value === "all" ? undefined : (value as TaskRunStatus))
            }
          >
            <SelectTrigger className="min-w-[160px] rounded-full bg-background/80">
              <SelectValue placeholder="Task status" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All statuses</SelectItem>
              {taskStatuses.map((item) => (
                <SelectItem key={item} value={item}>
                  {formatTaskStatus(item)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Select
            value={trigger ?? "all"}
            onValueChange={(value) =>
              onTriggerChange(value === "all" ? undefined : (value as TaskRunTrigger))
            }
          >
            <SelectTrigger className="min-w-[160px] rounded-full bg-background/80">
              <SelectValue placeholder="Task trigger" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All triggers</SelectItem>
              {taskTriggers.map((item) => (
                <SelectItem key={item} value={item}>
                  {formatTaskTrigger(item)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Label className="flex items-center gap-2 rounded-full border border-border/70 bg-background/80 px-3 py-2 text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
            <Checkbox
              checked={runningOnly}
              onCheckedChange={(checked) => onRunningOnlyChange(checked === true)}
            />
            Running only
          </Label>
        </div>
      </div>
    </div>
  );
}
