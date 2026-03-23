import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { useOutletContext } from "react-router-dom";
import { useTaskEvents } from "@/hooks/use-task-events";
import { ApiError, api } from "@/lib/api";
import { filterTaskListResponse } from "@/lib/tasks";
import type { TaskRunKind, TaskRunStatus, TaskRunTrigger } from "@/lib/types";
import { TasksPage } from "@/pages/TasksPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const DEFAULT_TASK_HISTORY_WINDOW_SEC = 7 * 24 * 60 * 60;
const TASK_HISTORY_WINDOW_REFRESH_INTERVAL_MS = 60 * 1000;

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function TasksRoute() {
  const { profileId, authMe, currentUser } = useOutletContext<RootOutletContext>();
  const [scope, setScope] = useState<"current" | "all">("current");
  const [kind, setKind] = useState<TaskRunKind | undefined>(undefined);
  const [status, setStatus] = useState<TaskRunStatus | undefined>(undefined);
  const [trigger, setTrigger] = useState<TaskRunTrigger | undefined>(undefined);
  const [runningOnly, setRunningOnly] = useState(false);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [taskWindowNowSec, setTaskWindowNowSec] = useState(() => Math.floor(Date.now() / 1000));
  const lastTaskQuerySignature = useRef<string | null>(null);
  const canAccess =
    currentUser.status === "resolved" ? currentUser.identity.is_admin : Boolean(authMe?.is_admin);
  const accessDenied =
    currentUser.status === "anonymous" ||
    (currentUser.status === "resolved" && !currentUser.identity.is_admin);
  const authError = currentUser.status === "error" ? currentUser.message : null;

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      setTaskWindowNowSec(Math.floor(Date.now() / 1000));
    }, TASK_HISTORY_WINDOW_REFRESH_INTERVAL_MS);

    return () => window.clearInterval(intervalId);
  }, []);

  const liveTaskQuery = useMemo(
    () => ({
      profile_id: scope === "current" ? profileId : undefined,
      kind,
      status,
      trigger,
      running_only: runningOnly,
    }),
    [kind, profileId, runningOnly, scope, status, trigger],
  );
  const visibleTaskQuery = useMemo(
    () => ({
      ...liveTaskQuery,
      since: taskWindowNowSec - DEFAULT_TASK_HISTORY_WINDOW_SEC,
    }),
    [liveTaskQuery, taskWindowNowSec],
  );
  const selectionResetSignature = useMemo(
    () =>
      JSON.stringify({
        profile_id: liveTaskQuery.profile_id,
        kind: liveTaskQuery.kind,
        status: liveTaskQuery.status,
        trigger: liveTaskQuery.trigger,
        running_only: liveTaskQuery.running_only,
      }),
    [
      liveTaskQuery.kind,
      liveTaskQuery.profile_id,
      liveTaskQuery.running_only,
      liveTaskQuery.status,
      liveTaskQuery.trigger,
    ],
  );

  const tasksQuery = useQuery({
    queryKey: ["tasks", liveTaskQuery],
    queryFn: () => api.listTasks(liveTaskQuery),
    enabled: canAccess,
  });
  const detailQuery = useQuery({
    queryKey: ["task-run", selectedRunId],
    queryFn: () => api.getTaskRunDetail(selectedRunId ?? ""),
    enabled: canAccess && Boolean(selectedRunId),
  });
  const streamState = useTaskEvents({
    query: liveTaskQuery,
    enabled: canAccess,
  });
  const visibleTaskList = useMemo(
    () => filterTaskListResponse(tasksQuery.data ?? null, visibleTaskQuery),
    [tasksQuery.data, visibleTaskQuery],
  );

  useEffect(() => {
    const runs = visibleTaskList?.runs ?? [];
    if (!runs.length) {
      setSelectedRunId(null);
      return;
    }
    if (!selectedRunId || !runs.some((run) => run.run_id === selectedRunId)) {
      setSelectedRunId(runs[0]?.run_id ?? null);
    }
  }, [selectedRunId, visibleTaskList?.runs]);

  useEffect(() => {
    if (lastTaskQuerySignature.current === null) {
      lastTaskQuerySignature.current = selectionResetSignature;
      return;
    }
    if (lastTaskQuerySignature.current !== selectionResetSignature) {
      lastTaskQuerySignature.current = selectionResetSignature;
      setSelectedRunId(null);
    }
  }, [selectionResetSignature]);

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
      streamState={authError ? "reconnecting" : streamState}
      taskError={authError ?? (tasksQuery.isError ? getErrorMessage(tasksQuery.error) : null)}
      taskList={visibleTaskList}
      tasksLoading={tasksQuery.isLoading}
      trigger={trigger}
    />
  );
}
