import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useOutletContext } from "react-router-dom";
import { useTaskEvents } from "@/hooks/use-task-events";
import { ApiError, api } from "@/lib/api";
import type { TaskRunKind, TaskRunStatus, TaskRunTrigger } from "@/lib/types";
import { TasksPage } from "@/pages/TasksPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function TasksRoute() {
  const { profileId, authMe } = useOutletContext<RootOutletContext>();
  const [scope, setScope] = useState<"current" | "all">("current");
  const [kind, setKind] = useState<TaskRunKind | undefined>(undefined);
  const [status, setStatus] = useState<TaskRunStatus | undefined>(undefined);
  const [trigger, setTrigger] = useState<TaskRunTrigger | undefined>(undefined);
  const [runningOnly, setRunningOnly] = useState(false);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const canAccess = Boolean(authMe?.is_admin);
  const accessDenied = authMe ? !authMe.is_admin : false;

  const taskQuery = useMemo(
    () => ({
      profile_id: scope === "current" ? profileId : undefined,
      kind,
      status,
      trigger,
      running_only: runningOnly,
      limit: 40,
    }),
    [kind, profileId, runningOnly, scope, status, trigger],
  );

  const tasksQuery = useQuery({
    queryKey: ["tasks", taskQuery],
    queryFn: () => api.listTasks(taskQuery),
    enabled: canAccess,
  });
  const detailQuery = useQuery({
    queryKey: ["task-run", selectedRunId],
    queryFn: () => api.getTaskRunDetail(selectedRunId ?? ""),
    enabled: canAccess && Boolean(selectedRunId),
  });
  const streamState = useTaskEvents({
    query: taskQuery,
    enabled: canAccess,
  });

  useEffect(() => {
    const runs = tasksQuery.data?.runs ?? [];
    if (!runs.length) {
      setSelectedRunId(null);
      return;
    }
    if (!selectedRunId || !runs.some((run) => run.run_id === selectedRunId)) {
      setSelectedRunId(runs[0]?.run_id ?? null);
    }
  }, [selectedRunId, tasksQuery.data?.runs]);

  useEffect(() => {
    if (scope === "current") {
      setSelectedRunId(null);
    }
  }, [scope]);

  return (
    <TasksPage
      accessDenied={accessDenied}
      detailError={detailQuery.isError ? getErrorMessage(detailQuery.error) : null}
      kind={kind}
      onKindChange={setKind}
      onRunningOnlyChange={setRunningOnly}
      onScopeChange={setScope}
      onSelectRun={setSelectedRunId}
      onStatusChange={setStatus}
      onTriggerChange={setTrigger}
      profileId={profileId}
      runningOnly={runningOnly}
      scope={scope}
      selectedRunDetail={detailQuery.data ?? null}
      selectedRunId={selectedRunId}
      selectedRunLoading={detailQuery.isLoading}
      status={status}
      streamState={streamState}
      taskError={tasksQuery.isError ? getErrorMessage(tasksQuery.error) : null}
      taskList={tasksQuery.data ?? null}
      tasksLoading={tasksQuery.isLoading}
      trigger={trigger}
    />
  );
}
