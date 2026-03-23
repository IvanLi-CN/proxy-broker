import type {
  TaskEventLevel,
  TaskRunKind,
  TaskRunStage,
  TaskRunStatus,
  TaskRunTrigger,
} from "@/lib/types";

export function formatTaskKind(kind: TaskRunKind) {
  switch (kind) {
    case "subscription_sync":
      return "Subscription sync";
    case "metadata_refresh_incremental":
      return "Incremental refresh";
    case "metadata_refresh_full":
      return "Full refresh";
  }
}

export function formatTaskTrigger(trigger: TaskRunTrigger) {
  switch (trigger) {
    case "schedule":
      return "Scheduled";
    case "post_load":
      return "Post-load";
  }
}

export function formatTaskStatus(status: TaskRunStatus) {
  switch (status) {
    case "queued":
      return "Queued";
    case "running":
      return "Running";
    case "succeeded":
      return "Succeeded";
    case "failed":
      return "Failed";
    case "skipped":
      return "Skipped";
  }
}

export function formatTaskStage(stage: TaskRunStage) {
  switch (stage) {
    case "queued":
      return "Queued";
    case "loading_subscription":
      return "Loading subscription";
    case "diffing_inventory":
      return "Diffing inventory";
    case "probing":
      return "Probing";
    case "geo_enrichment":
      return "Geo enrichment";
    case "persisting":
      return "Persisting";
    case "completed":
      return "Completed";
  }
}

export function formatTaskEventLevel(level: TaskEventLevel) {
  switch (level) {
    case "info":
      return "Info";
    case "warning":
      return "Warning";
    case "error":
      return "Error";
  }
}

export function formatTaskProgress(current?: number | null, total?: number | null) {
  if (total == null) {
    return current != null ? `${current}` : "N/A";
  }
  return `${current ?? 0}/${total}`;
}
