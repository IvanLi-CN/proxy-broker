import type { Locale, Translator } from "@/i18n";
import type {
  TaskEventLevel,
  TaskRunKind,
  TaskRunStage,
  TaskRunStatus,
  TaskRunTrigger,
} from "@/lib/types";

const taskPayloadLabelMap: Record<string, string> = {
  targeted_ips: "Targeted IPs",
  probed_ips: "Probed IPs",
  geo_updated: "Geo records updated",
  skipped_cached: "Cached entries skipped",
  loaded_proxies: "Loaded proxies",
  distinct_ips: "Distinct IPs",
  reason: "Reason",
};

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

export function formatTaskPayloadKey(key: string, t: Translator) {
  return t(taskPayloadLabelMap[key] ?? key);
}

export function localizeTaskPayload(value: unknown, t: Translator): unknown {
  if (Array.isArray(value)) {
    return value.map((item) => localizeTaskPayload(item, t));
  }

  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([key, nestedValue]) => [
        formatTaskPayloadKey(key, t),
        localizeTaskPayload(nestedValue, t),
      ]),
    );
  }

  return value;
}

export function formatTaskEventMessage(message: string, t: Translator) {
  switch (message) {
    case "Refreshing subscription feed for profile.":
    case "Refreshing probe metadata.":
    case "Task run queued.":
    case "Task run completed successfully.":
    case "Task run skipped.":
    case "Task run failed.":
    case "Task run is running.":
      return t(message);
  }

  const syncFinishedMatch = message.match(/^Subscription sync finished with (\d+) new IP\(s\)\.$/);
  if (syncFinishedMatch) {
    return t("Subscription sync finished with {count} new IPs.", {
      count: syncFinishedMatch[1] ?? "0",
    });
  }

  return message;
}
