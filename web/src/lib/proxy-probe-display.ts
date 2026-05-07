export type ProbeDisplayState = "success" | "failed" | "empty";

export function probeLatencyToneClass(
  state: ProbeDisplayState,
  latencyMs: number | null | undefined,
) {
  if (state === "failed") {
    return "text-rose-700 dark:text-rose-300";
  }

  if (state === "empty" || latencyMs == null) {
    return "text-muted-foreground";
  }

  if (latencyMs <= 150) {
    return "text-emerald-700 dark:text-emerald-300";
  }

  if (latencyMs <= 300) {
    return "text-amber-700 dark:text-amber-300";
  }

  return "text-rose-700 dark:text-rose-300";
}

export function probeLatencyBadgeToneClass(
  state: ProbeDisplayState,
  latencyMs: number | null | undefined,
) {
  if (state === "failed") {
    return "border-rose-700/25 bg-rose-50 text-rose-800 dark:border-rose-300/25 dark:bg-rose-950/40 dark:text-rose-200";
  }

  if (state === "empty" || latencyMs == null) {
    return "border-border bg-muted/50 text-muted-foreground";
  }

  if (latencyMs <= 150) {
    return "border-emerald-700/25 bg-emerald-50 text-emerald-800 dark:border-emerald-300/25 dark:bg-emerald-950/40 dark:text-emerald-200";
  }

  if (latencyMs <= 300) {
    return "border-amber-700/25 bg-amber-50 text-amber-800 dark:border-amber-300/25 dark:bg-amber-950/40 dark:text-amber-200";
  }

  return "border-rose-700/25 bg-rose-50 text-rose-800 dark:border-rose-300/25 dark:bg-rose-950/40 dark:text-rose-200";
}
