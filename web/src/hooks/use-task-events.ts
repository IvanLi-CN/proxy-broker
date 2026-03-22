import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { api } from "@/lib/api";
import { matchesTaskQuery, parseTaskEnvelope, sortTaskRuns } from "@/lib/tasks";
import type {
  TaskListQuery,
  TaskListResponse,
  TaskRunDetail,
  TaskRunEvent,
  TaskRunSummary,
  TaskSummary,
} from "@/lib/types";

export type TaskStreamState = "connecting" | "live" | "reconnecting";

interface UseTaskEventsOptions {
  query: TaskListQuery;
  enabled?: boolean;
}

export function useTaskEvents({ query, enabled = true }: UseTaskEventsOptions) {
  const queryClient = useQueryClient();
  const [state, setState] = useState<TaskStreamState>("connecting");

  useEffect(() => {
    if (!enabled) {
      return undefined;
    }

    const source = new EventSource(api.getTaskEventsUrl(query));
    const listQueryKey = ["tasks", query] as const;

    const handleOpen = () => {
      setState("live");
    };
    const handleError = () => {
      setState("reconnecting");
    };
    const handleSnapshot = (event: MessageEvent<string>) => {
      const envelope = parseTaskEnvelope<TaskListResponse>(event.data);
      queryClient.setQueryData(listQueryKey, envelope.data);
      setState("live");
    };
    const handleRunUpsert = (event: MessageEvent<string>) => {
      const envelope = parseTaskEnvelope<TaskRunSummary>(event.data);
      const run = envelope.data;

      queryClient.setQueryData<TaskListResponse>(listQueryKey, (current) => {
        if (!current) {
          return current;
        }

        let runs = current.runs.filter((item) => item.run_id !== run.run_id);
        if (matchesTaskQuery(run, query)) {
          runs = sortTaskRuns([...runs, run]);
        }
        if (query.limit != null) {
          runs = runs.slice(0, query.limit);
        }

        return {
          ...current,
          runs,
        };
      });

      queryClient.setQueryData<TaskRunDetail>(["task-run", run.run_id], (current) =>
        current ? { ...current, run } : current,
      );
      setState("live");
    };
    const handleRunEvent = (event: MessageEvent<string>) => {
      const envelope = parseTaskEnvelope<TaskRunEvent>(event.data);
      const taskEvent = envelope.data;

      queryClient.setQueryData<TaskRunDetail>(["task-run", taskEvent.run_id], (current) =>
        current
          ? {
              ...current,
              events: [...current.events, taskEvent].sort((left, right) => {
                if (left.at !== right.at) {
                  return left.at - right.at;
                }
                return left.event_id.localeCompare(right.event_id);
              }),
            }
          : current,
      );
      setState("live");
    };
    const handleSummary = (event: MessageEvent<string>) => {
      const envelope = parseTaskEnvelope<TaskSummary>(event.data);
      queryClient.setQueryData<TaskListResponse>(listQueryKey, (current) =>
        current ? { ...current, summary: envelope.data } : current,
      );
      setState("live");
    };
    const handleHeartbeat = () => {
      setState("live");
    };

    source.addEventListener("open", handleOpen as EventListener);
    source.addEventListener("error", handleError as EventListener);
    source.addEventListener("snapshot", handleSnapshot as EventListener);
    source.addEventListener("run-upsert", handleRunUpsert as EventListener);
    source.addEventListener("run-event", handleRunEvent as EventListener);
    source.addEventListener("summary", handleSummary as EventListener);
    source.addEventListener("heartbeat", handleHeartbeat as EventListener);

    return () => {
      source.removeEventListener("open", handleOpen as EventListener);
      source.removeEventListener("error", handleError as EventListener);
      source.removeEventListener("snapshot", handleSnapshot as EventListener);
      source.removeEventListener("run-upsert", handleRunUpsert as EventListener);
      source.removeEventListener("run-event", handleRunEvent as EventListener);
      source.removeEventListener("summary", handleSummary as EventListener);
      source.removeEventListener("heartbeat", handleHeartbeat as EventListener);
      source.close();
    };
  }, [enabled, query, queryClient]);

  return state;
}
