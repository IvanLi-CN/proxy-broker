import { ArrowRightIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useI18n } from "@/i18n";
import { formatTaskErrorMessage } from "@/lib/error-messages";
import { formatTimestamp } from "@/lib/format";
import {
  formatTaskKind,
  formatTaskProgress,
  formatTaskStage,
  formatTaskStatus,
  formatTaskTrigger,
} from "@/lib/tasks-view";
import type { TaskRunSummary } from "@/lib/types";
import { cn } from "@/lib/utils";

interface TasksTableProps {
  runs: TaskRunSummary[];
  isLoading: boolean;
  selectedRunId?: string | null;
  onSelectRun: (runId: string) => void;
}

const statusTone = {
  queued: "border-slate-400/25 bg-slate-400/10 text-slate-700 dark:text-slate-300",
  running: "border-sky-500/25 bg-sky-500/[0.12] text-sky-700 dark:text-sky-300",
  succeeded: "border-emerald-500/25 bg-emerald-500/[0.12] text-emerald-700 dark:text-emerald-300",
  failed: "border-rose-500/25 bg-rose-500/[0.12] text-rose-700 dark:text-rose-300",
  skipped: "border-amber-500/25 bg-amber-500/[0.12] text-amber-700 dark:text-amber-300",
} as const;

export function TasksTable({ runs, isLoading, selectedRunId, onSelectRun }: TasksTableProps) {
  const { locale, t } = useI18n();

  if (!runs.length && !isLoading) {
    return (
      <div className="rounded-[24px] border border-dashed border-border/70 bg-muted/10 px-6 py-12 text-center">
        <div className="text-base font-semibold text-foreground">
          {t("No task runs match this view")}
        </div>
        <div className="mt-2 text-sm leading-6 text-muted-foreground">
          {t(
            "Narrow the filters less aggressively or wait for the next scheduled subscription sync.",
          )}
        </div>
      </div>
    );
  }

  return (
    <Table className="min-w-[980px]">
      <TableHeader>
        <TableRow className="border-b border-border/70 bg-muted/20">
          <TableHead className="px-4">{t("Profile")}</TableHead>
          <TableHead>{t("Task")}</TableHead>
          <TableHead>{t("Status")}</TableHead>
          <TableHead>{t("Stage")}</TableHead>
          <TableHead>{t("Progress")}</TableHead>
          <TableHead>{t("Latest note")}</TableHead>
          <TableHead className="pr-4 text-right">{t("Timeline")}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {runs.map((run) => (
          <TableRow
            key={run.run_id}
            className={cn(
              "cursor-pointer transition-colors hover:bg-muted/20 [&_td]:py-3",
              selectedRunId === run.run_id && "bg-primary/[0.06]",
            )}
            onClick={() => onSelectRun(run.run_id)}
          >
            <TableCell className="px-4">
              <div className="font-mono text-xs md:text-sm">{run.profile_id}</div>
            </TableCell>
            <TableCell>
              <div className="space-y-1">
                <div className="text-sm font-semibold text-foreground">
                  {formatTaskKind(run.kind, t)}
                </div>
                <div className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
                  {formatTaskTrigger(run.trigger, t)}
                </div>
              </div>
            </TableCell>
            <TableCell>
              <Badge
                variant="outline"
                className={`rounded-full px-3 py-1 text-[11px] ${statusTone[run.status]}`}
              >
                {formatTaskStatus(run.status, t)}
              </Badge>
            </TableCell>
            <TableCell className="text-sm text-muted-foreground">
              {formatTaskStage(run.stage, t)}
            </TableCell>
            <TableCell className="font-mono text-xs md:text-sm">
              {formatTaskProgress(locale, t, run.progress_current, run.progress_total)}
            </TableCell>
            <TableCell className="max-w-[260px] text-sm text-muted-foreground">
              {run.error_code || run.error_message || typeof run.summary_json?.reason === "string"
                ? formatTaskErrorMessage(run, t)
                : t("Open the event stream for the full run log.")}
            </TableCell>
            <TableCell className="pr-4 text-right">
              <div className="space-y-1 text-xs text-muted-foreground">
                <div>{formatTimestamp(locale, t, run.started_at ?? run.created_at)}</div>
                <div className="inline-flex items-center gap-1 font-medium text-primary">
                  {t("Inspect")}
                  <ArrowRightIcon className="size-3.5" />
                </div>
              </div>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
