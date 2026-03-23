import { ActivityIcon, CircleAlertIcon, ScrollTextIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { formatTimestamp } from "@/lib/format";
import {
  formatTaskEventLevel,
  formatTaskKind,
  formatTaskProgress,
  formatTaskStage,
  formatTaskStatus,
  formatTaskTrigger,
} from "@/lib/tasks-view";
import type { TaskRunDetail } from "@/lib/types";

interface TaskRunDetailPanelProps {
  detail?: TaskRunDetail | null;
  isLoading: boolean;
}

const eventTone = {
  info: "border-sky-500/20 bg-sky-500/[0.08] text-sky-700 dark:text-sky-300",
  warning: "border-amber-500/20 bg-amber-500/[0.09] text-amber-700 dark:text-amber-300",
  error: "border-rose-500/20 bg-rose-500/[0.08] text-rose-700 dark:text-rose-300",
} as const;

export function TaskRunDetailPanel({ detail, isLoading }: TaskRunDetailPanelProps) {
  if (!detail && !isLoading) {
    return (
      <Card className="border-border/70 bg-card/96 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
        <CardHeader className="space-y-3 border-b border-border/70 pb-5">
          <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
            Run detail
          </div>
          <CardTitle className="text-xl tracking-tight">Pick a task run to inspect</CardTitle>
          <CardDescription className="text-sm leading-6 text-muted-foreground">
            The right rail shows the latest summary payload and the event stream for the selected
            run.
          </CardDescription>
        </CardHeader>
      </Card>
    );
  }

  const run = detail?.run;

  return (
    <Card className="border-border/70 bg-card/96 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
      <CardHeader className="space-y-3 border-b border-border/70 pb-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-2">
            <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
              Run detail
            </div>
            <CardTitle className="text-xl tracking-tight">
              {run ? formatTaskKind(run.kind) : "Loading task run"}
            </CardTitle>
            <CardDescription className="text-sm leading-6 text-muted-foreground">
              {run
                ? `${formatTaskTrigger(run.trigger)} for ${run.profile_id}`
                : "Waiting for the run payload."}
            </CardDescription>
          </div>
          {run ? (
            <Badge variant="outline" className="rounded-full px-3 py-1 text-[11px]">
              {formatTaskStatus(run.status)}
            </Badge>
          ) : null}
        </div>
      </CardHeader>
      <CardContent className="space-y-5 pt-5">
        {run ? (
          <>
            <div className="grid gap-3 md:grid-cols-2">
              <div className="rounded-2xl border border-border/70 bg-background/80 p-4">
                <div className="text-[11px] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                  Stage
                </div>
                <div className="mt-2 text-base font-semibold text-foreground">
                  {formatTaskStage(run.stage)}
                </div>
                <div className="mt-2 text-sm text-muted-foreground">
                  Progress {formatTaskProgress(run.progress_current, run.progress_total)}
                </div>
              </div>
              <div className="rounded-2xl border border-border/70 bg-background/80 p-4">
                <div className="text-[11px] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                  Timeline
                </div>
                <div className="mt-2 space-y-1 text-sm text-muted-foreground">
                  <div>Queued {formatTimestamp(run.created_at)}</div>
                  <div>Started {formatTimestamp(run.started_at)}</div>
                  <div>Finished {formatTimestamp(run.finished_at)}</div>
                </div>
              </div>
            </div>

            {run.summary_json ? (
              <div className="rounded-2xl border border-border/70 bg-muted/15 p-4">
                <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                  <ActivityIcon className="size-4 text-primary" />
                  Summary payload
                </div>
                <div className="mt-3 grid gap-2 text-sm text-muted-foreground">
                  {Object.entries(run.summary_json).map(([key, value]) => (
                    <div
                      key={key}
                      className="flex items-start justify-between gap-4 rounded-xl bg-background/80 px-3 py-2"
                    >
                      <span className="font-medium text-foreground">{key}</span>
                      <span className="max-w-[60%] text-right font-mono text-xs">
                        {typeof value === "string" || typeof value === "number"
                          ? String(value)
                          : JSON.stringify(value)}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}

            {run.error_message ? (
              <div className="rounded-2xl border border-rose-500/20 bg-rose-500/[0.08] p-4">
                <div className="flex items-center gap-2 text-sm font-semibold text-rose-700 dark:text-rose-300">
                  <CircleAlertIcon className="size-4" />
                  Failure detail
                </div>
                <div className="mt-2 text-sm leading-6 text-rose-700/90 dark:text-rose-200">
                  {run.error_message}
                </div>
              </div>
            ) : null}
          </>
        ) : null}

        <div className="rounded-2xl border border-border/70 bg-background/70">
          <div className="flex items-center gap-2 border-b border-border/70 px-4 py-3 text-sm font-semibold text-foreground">
            <ScrollTextIcon className="size-4 text-primary" />
            Event stream
          </div>
          <ScrollArea className="h-[420px]">
            <div className="space-y-3 p-4">
              {detail?.events.length ? (
                detail.events.map((event) => (
                  <div
                    key={event.event_id}
                    className="rounded-2xl border border-border/70 bg-card/90 p-4"
                  >
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <Badge
                        variant="outline"
                        className={`rounded-full px-3 py-1 text-[11px] ${eventTone[event.level]}`}
                      >
                        {formatTaskEventLevel(event.level)}
                      </Badge>
                      <div className="text-xs text-muted-foreground">
                        {formatTimestamp(event.at)}
                      </div>
                    </div>
                    <div className="mt-3 text-sm font-semibold text-foreground">
                      {event.message}
                    </div>
                    <div className="mt-1 text-xs uppercase tracking-[0.16em] text-muted-foreground">
                      {formatTaskStage(event.stage)}
                    </div>
                    {event.payload_json ? (
                      <pre className="mt-3 overflow-x-auto rounded-xl bg-background/85 p-3 text-xs text-muted-foreground">
                        {JSON.stringify(event.payload_json, null, 2)}
                      </pre>
                    ) : null}
                  </div>
                ))
              ) : (
                <div className="px-2 py-10 text-center text-sm text-muted-foreground">
                  {isLoading ? "Loading task events..." : "No task events recorded yet."}
                </div>
              )}
            </div>
          </ScrollArea>
        </div>
      </CardContent>
    </Card>
  );
}
