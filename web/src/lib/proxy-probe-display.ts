export type ProbeDisplayState = "success" | "failed" | "empty";

export function probeLatencyToneClass(
  state: ProbeDisplayState,
  latencyMs: number | null | undefined,
) {
  if (state === "failed") {
    return "text-destructive";
  }

  if (state === "empty" || latencyMs == null) {
    return "text-muted-foreground";
  }

  if (latencyMs <= 150) {
    return "text-chart-5";
  }

  if (latencyMs <= 300) {
    return "text-chart-3";
  }

  return "text-destructive";
}

export function probeLatencyBadgeToneClass(
  state: ProbeDisplayState,
  latencyMs: number | null | undefined,
) {
  if (state === "failed") {
    return "border-destructive/20 bg-destructive/10 text-destructive";
  }

  if (state === "empty" || latencyMs == null) {
    return "border-border bg-muted/50 text-muted-foreground";
  }

  if (latencyMs <= 150) {
    return "border-chart-5/25 bg-chart-5/10 text-chart-5";
  }

  if (latencyMs <= 300) {
    return "border-chart-3/25 bg-chart-3/10 text-chart-3";
  }

  return "border-destructive/20 bg-destructive/10 text-destructive";
}
