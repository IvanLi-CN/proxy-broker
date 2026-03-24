import type { Locale, Translator } from "@/i18n";
import type {
  TaskEventLevel,
  TaskRunKind,
  TaskRunStage,
  TaskRunStatus,
  TaskRunTrigger,
} from "@/lib/types";

export function formatTaskKind(kind: TaskRunKind, t: Translator) {
  switch (kind) {
    case "subscription_sync":
      return t("Subscription sync");
    case "metadata_refresh_incremental":
      return t("Incremental refresh");
    case "metadata_refresh_full":
      return t("Full refresh");
  }
}

export function formatTaskTrigger(trigger: TaskRunTrigger, t: Translator) {
  switch (trigger) {
    case "schedule":
      return t("Scheduled");
    case "post_load":
      return t("Post-load");
  }
}

export function formatTaskStatus(status: TaskRunStatus, t: Translator) {
  switch (status) {
    case "queued":
      return t("Queued");
    case "running":
      return t("Running");
    case "succeeded":
      return t("Succeeded");
    case "failed":
      return t("Failed");
    case "skipped":
      return t("Skipped");
  }
}

export function formatTaskStage(stage: TaskRunStage, t: Translator) {
  switch (stage) {
    case "queued":
      return t("Queued");
    case "loading_subscription":
      return t("Loading subscription");
    case "diffing_inventory":
      return t("Diffing inventory");
    case "probing":
      return t("Probing");
    case "geo_enrichment":
      return t("Geo enrichment");
    case "persisting":
      return t("Persisting");
    case "completed":
      return t("Completed");
  }
}

export function formatTaskEventLevel(level: TaskEventLevel, t: Translator) {
  switch (level) {
    case "info":
      return t("Info");
    case "warning":
      return t("Warning");
    case "error":
      return t("Error");
  }
}

export function formatTaskProgress(
  locale: Locale,
  t: Translator,
  current?: number | null,
  total?: number | null,
) {
  const formatter = new Intl.NumberFormat(locale);
  if (total == null) {
    return current != null ? formatter.format(current) : t("N/A");
  }
  return `${formatter.format(current ?? 0)}/${formatter.format(total)}`;
}
