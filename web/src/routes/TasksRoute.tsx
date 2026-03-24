import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useOutletContext } from "react-router-dom";

import { useTaskEvents } from "@/hooks/use-task-events";
import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import { filterTaskListResponse } from "@/lib/tasks";
import type { TaskRunKind, TaskRunStatus, TaskRunTrigger } from "@/lib/types";
import { TasksPage } from "@/pages/TasksPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const DEFAULT_TASK_HISTORY_WINDOW_SEC = 7 * 24 * 60 * 60;
const TASK_HISTORY_WINDOW_REFRESH_INTERVAL_MS = 60 * 1000;
const TASK_HISTORY_QUERY_REBASE_INTERVAL_MS = 60 * 60 * 1000;

export function TasksRoute() {
  const { t } = useI18n();
  const { profileId, authMe, currentUser } = useOutletContext<RootOutletContext>();
  const [scope, setScope] = useState<"current" | "all">("current");
  const [kind, setKind] = useState<TaskRunKind | undefined>(undefined);
  const [status, setStatus] = useState<TaskRunStatus | undefined>(undefined);
  const [trigger, setTrigger] = useState<TaskRunTrigger | undefined>(undefined);
  const [runningOnly, setRunningOnly] = useState(false);
  const [selectedRun, setSelectedRun] = useState<{
    viewSignature: string | null;
    runId: string | null;
  }>({
    viewSignature: null,
    runId: null,
  });
  const [taskWindowNowSec, setTaskWindowNowSec] = useState(() => Math.floor(Date.now() / 1000));
  const [taskQueryNowSec, setTaskQueryNowSec] = useState(() => Math.floor(Date.now() / 1000));
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

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      setTaskQueryNowSec(Math.floor(Date.now() / 1000));
    }, TASK_HISTORY_QUERY_REBASE_INTERVAL_MS);

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
  const requestTaskQuery = useMemo(
    () => ({
      ...liveTaskQuery,
      since: taskQueryNowSec - DEFAULT_TASK_HISTORY_WINDOW_SEC,
    }),
    [liveTaskQuery, taskQueryNowSec],
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
  const selectedRunId =
    selectedRun.viewSignature === selectionResetSignature ? selectedRun.runId : null;

  const tasksQuery = useQuery({
    queryKey: ["tasks", requestTaskQuery],
    queryFn: () => api.listTasks(requestTaskQuery),
    enabled: canAccess,
    placeholderData: (previousData) => previousData,
  });
  const detailQuery = useQuery({
    queryKey: ["task-run", selectedRunId],
    queryFn: () => api.getTaskRunDetail(selectedRunId ?? ""),
    enabled: canAccess && Boolean(selectedRunId),
  });
  const streamState = useTaskEvents({
    query: requestTaskQuery,
    enabled: canAccess,
  });
  const visibleTaskList = useMemo(
    () => filterTaskListResponse(tasksQuery.data ?? null, visibleTaskQuery),
    [tasksQuery.data, visibleTaskQuery],
  );

  useEffect(() => {
    const runs = visibleTaskList?.runs ?? [];
    if (!runs.length) {
      setSelectedRun({
        viewSignature: selectionResetSignature,
        runId: null,
      });
      return;
    }
    if (!selectedRunId || !runs.some((run) => run.run_id === selectedRunId)) {
      setSelectedRun({
        viewSignature: selectionResetSignature,
        runId: runs[0]?.run_id ?? null,
      });
    }
  }, [selectedRunId, selectionResetSignature, visibleTaskList?.runs]);

  return (
    <TasksPage
      accessDenied={accessDenied}
      detailError={detailQuery.isError ? formatApiErrorMessage(detailQuery.error, t) : null}
      kind={kind}
      onKindChange={setKind}
      onRunningOnlyChange={setRunningOnly}
      onScopeChange={setScope}
      onSelectRun={(runId) =>
        setSelectedRun({
          viewSignature: selectionResetSignature,
          runId,
        })
      }
      onStatusChange={setStatus}
      onTriggerChange={setTrigger}
      profileId={profileId}
      runningOnly={runningOnly}
      scope={scope}
      selectedRunDetail={selectedRunId ? (detailQuery.data ?? null) : null}
      selectedRunId={selectedRunId}
      selectedRunLoading={Boolean(selectedRunId) && detailQuery.isLoading}
      status={status}
      streamState={authError ? "reconnecting" : streamState}
      taskError={
        authError ?? (tasksQuery.isError ? formatApiErrorMessage(tasksQuery.error, t) : null)
      }
      taskList={visibleTaskList}
      tasksLoading={tasksQuery.isLoading}
      trigger={trigger}
    />
  );
}
