import { RadioTowerIcon, Rows3Icon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { DataTablePanel } from "@/components/DataTablePanel";
import { RouteHero } from "@/components/RouteHero";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { WorkflowRail } from "@/components/WorkflowRail";
import { TaskFiltersBar } from "@/features/tasks/components/TaskFiltersBar";
import { TaskRunDetailPanel } from "@/features/tasks/components/TaskRunDetailPanel";
import { TaskSummaryCards } from "@/features/tasks/components/TaskSummaryCards";
import { TasksTable } from "@/features/tasks/components/TasksTable";
import { useI18n } from "@/i18n";
import type {
  TaskListResponse,
  TaskRunDetail,
  TaskRunKind,
  TaskRunStatus,
  TaskRunTrigger,
} from "@/lib/types";

interface TasksPageProps {
  profileId: string;
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
  taskList?: TaskListResponse | null;
  tasksLoading: boolean;
  taskError?: string | null;
  streamState: "connecting" | "live" | "reconnecting";
  selectedRunId?: string | null;
  onSelectRun: (runId: string) => void;
  selectedRunDetail?: TaskRunDetail | null;
  selectedRunLoading: boolean;
  detailError?: string | null;
  accessDenied?: boolean;
}

export function TasksPage({
  profileId,
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
  taskList,
  tasksLoading,
  taskError,
  streamState,
  selectedRunId,
  onSelectRun,
  selectedRunDetail,
  selectedRunLoading,
  detailError,
  accessDenied = false,
}: TasksPageProps) {
  const { formatNumber, t } = useI18n();
  const runs = taskList?.runs ?? [];
  const summary = taskList?.summary ?? {
    total_runs: 0,
    queued_runs: 0,
    running_runs: 0,
    failed_runs: 0,
    succeeded_runs: 0,
    skipped_runs: 0,
    last_run_at: null,
  };

  return (
    <div className="space-y-8">
      <RouteHero
        eyebrow={t("Tasks")}
        title={t("Tasks hero title")}
        description={t("Tasks hero description")}
        badges={[
          {
            label: t("{count} running", { count: formatNumber(summary.running_runs) }),
            tone: summary.running_runs > 0 ? "warning" : "neutral",
          },
          {
            label: t("{count} failed", { count: formatNumber(summary.failed_runs) }),
            tone: summary.failed_runs > 0 ? "danger" : "positive",
          },
          {
            label:
              scope === "current" ? t("profile {profileId}", { profileId }) : t("all profiles"),
            tone: "neutral",
          },
        ]}
        aside={
          <WorkflowRail
            eyebrow={t("Realtime loop")}
            title={t("How the board should be read")}
            steps={[
              {
                title: t("Read the cadence"),
                description: t(
                  "Queued and running rows tell you whether the 10-minute sync lane is healthy.",
                ),
              },
              {
                title: t("Inspect the stream"),
                description: t(
                  "Open the selected run to read stage transitions and payload summaries without leaving the page.",
                ),
              },
              {
                title: t("Treat failures as operator signals"),
                description: t(
                  "A failed run should explain whether the source fetch, probing, or geo enrichment stalled.",
                ),
              },
            ]}
          />
        }
      />

      {accessDenied ? (
        <ActionResponsePanel
          title={t("Admin access required")}
          description={t(
            "The task center is currently restricted to the admin operator plane and development principal.",
          )}
          tone="error"
        />
      ) : (
        <>
          <TaskSummaryCards summary={summary} streamState={streamState} />

          <TaskFiltersBar
            kind={kind}
            onKindChange={onKindChange}
            onRunningOnlyChange={onRunningOnlyChange}
            onScopeChange={onScopeChange}
            onStatusChange={onStatusChange}
            onTriggerChange={onTriggerChange}
            runningOnly={runningOnly}
            scope={scope}
            status={status}
            trigger={trigger}
          />

          {streamState !== "live" ? (
            <Alert
              aria-live="polite"
              className="border-amber-500/20 bg-amber-500/[0.08]"
              role="alert"
            >
              <RadioTowerIcon className="size-4 text-amber-600" />
              <AlertTitle>
                {streamState === "connecting"
                  ? t("Connecting to the task stream")
                  : t("Task stream reconnecting")}
              </AlertTitle>
              <AlertDescription>
                {t("The page keeps the last snapshot visible while SSE catches up.")}
              </AlertDescription>
            </Alert>
          ) : null}

          {taskError ? (
            <ActionResponsePanel
              title={t("Task list failed")}
              description={taskError}
              tone="error"
            />
          ) : null}
          {detailError ? (
            <ActionResponsePanel
              title={t("Task detail failed")}
              description={detailError}
              tone="error"
            />
          ) : null}

          <section className="grid gap-6 xl:grid-cols-[minmax(0,1.15fr)_420px]">
            <DataTablePanel
              eyebrow={t("Run board")}
              title={t("Task history and current activity")}
              description={t(
                "Rows are kept hot by SSE, so stage changes and result summaries land without polling.",
              )}
              chips={[
                t(runs.length === 1 ? "{count} visible run" : "{count} visible runs", {
                  count: formatNumber(runs.length),
                }),
                runningOnly ? t("running-only filter") : t("history included"),
                selectedRunId
                  ? t("focused {runId}", { runId: selectedRunId })
                  : t("no run selected"),
              ]}
              actions={
                <Badge
                  variant="outline"
                  className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
                >
                  <Rows3Icon className="mr-1 size-3.5" />
                  {streamState === "live"
                    ? t("Live stream")
                    : streamState === "connecting"
                      ? t("Connecting to the task stream")
                      : t("Task stream reconnecting")}
                </Badge>
              }
            >
              <TasksTable
                isLoading={tasksLoading}
                onSelectRun={onSelectRun}
                runs={runs}
                selectedRunId={selectedRunId}
              />
            </DataTablePanel>

            <TaskRunDetailPanel detail={selectedRunDetail} isLoading={selectedRunLoading} />
          </section>
        </>
      )}
    </div>
  );
}
