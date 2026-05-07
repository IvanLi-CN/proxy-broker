import { useEffect, useMemo, useRef, useState } from "react";

import { api } from "@/lib/api";
import { parseTaskEnvelope } from "@/lib/tasks";
import type { TaskListResponse, TaskRunEvent, TaskRunSummary } from "@/lib/types";

const PROXY_KINDS = new Set(["proxy_metadata_refresh", "proxy_latency_probe"] as const);

type ProxyKind = "proxy_metadata_refresh" | "proxy_latency_probe";
export type ProxyOperationStreamState = "connecting" | "live" | "reconnecting";

export interface ProxyNodeLiveState {
  nodeId: string;
  runId: string;
  kind: ProxyKind;
  latestSampleMs?: number | null;
  latestRound?: number;
  samplesTotal?: number;
  progressCurrent?: number | null;
  progressTotal?: number | null;
  message?: string;
  at: number;
}

interface UseProxyOperationEventsOptions {
  projectId?: string | null;
  enabled?: boolean;
}

function isProxyRun(run: TaskRunSummary): run is TaskRunSummary & { kind: ProxyKind } {
  return PROXY_KINDS.has(run.kind as ProxyKind);
}

function isTerminal(status: TaskRunSummary["status"]) {
  return status === "succeeded" || status === "failed" || status === "skipped";
}

export function useProxyOperationEvents({
  projectId,
  enabled = true,
}: UseProxyOperationEventsOptions) {
  const [connectionState, setConnectionState] = useState<ProxyOperationStreamState>("connecting");
  const [runsById, setRunsById] = useState<Record<string, TaskRunSummary>>({});
  const [nodeStateById, setNodeStateById] = useState<Record<string, ProxyNodeLiveState>>({});
  const runsRef = useRef<Record<string, TaskRunSummary>>({});

  useEffect(() => {
    runsRef.current = runsById;
  }, [runsById]);

  useEffect(() => {
    if (!enabled || !projectId) {
      setRunsById({});
      setNodeStateById({});
      return undefined;
    }

    if (typeof EventSource === "undefined") {
      setConnectionState("reconnecting");
      return undefined;
    }

    const source = new EventSource(api.getTaskEventsUrl({ project_id: projectId, limit: 50 }));

    const handleOpen = () => setConnectionState("live");
    const handleError = () => setConnectionState("reconnecting");
    const handleSnapshot = (event: MessageEvent<string>) => {
      const envelope = parseTaskEnvelope<TaskListResponse>(event.data);
      const nextRuns = Object.fromEntries(
        envelope.data.runs.filter(isProxyRun).map((run) => [run.run_id, run]),
      );
      setRunsById(nextRuns);
      setConnectionState("live");
    };
    const handleRunUpsert = (event: MessageEvent<string>) => {
      const envelope = parseTaskEnvelope<TaskRunSummary>(event.data);
      const run = envelope.data;
      if (!isProxyRun(run)) {
        return;
      }
      setRunsById((current) => ({ ...current, [run.run_id]: run }));
      if (isTerminal(run.status)) {
        setNodeStateById((current) => {
          const next = { ...current };
          for (const [nodeId, state] of Object.entries(current)) {
            if (state.runId === run.run_id) {
              next[nodeId] = {
                ...state,
                progressCurrent: run.progress_current,
                progressTotal: run.progress_total,
                at: run.finished_at ?? state.at,
              };
            }
          }
          return next;
        });
      }
      setConnectionState("live");
    };
    const handleRunEvent = (event: MessageEvent<string>) => {
      const envelope = parseTaskEnvelope<TaskRunEvent>(event.data);
      const taskEvent = envelope.data;
      const payload = taskEvent.payload_json as Record<string, unknown> | undefined;
      const nodeId = typeof payload?.node_id === "string" ? payload.node_id : null;
      if (!nodeId) {
        return;
      }
      setNodeStateById((current) => ({
        ...current,
        [nodeId]: {
          nodeId,
          runId: taskEvent.run_id,
          kind:
            (runsRef.current[taskEvent.run_id]?.kind as ProxyKind | undefined) ??
            "proxy_metadata_refresh",
          latestSampleMs:
            typeof payload?.sample_ms === "number"
              ? payload.sample_ms
              : payload?.sample_ms === null
                ? null
                : undefined,
          latestRound: typeof payload?.round === "number" ? payload.round : undefined,
          samplesTotal:
            typeof payload?.samples_total === "number" ? payload.samples_total : undefined,
          progressCurrent:
            typeof payload?.progress_current === "number"
              ? payload.progress_current
              : runsRef.current[taskEvent.run_id]?.progress_current,
          progressTotal:
            typeof payload?.progress_total === "number"
              ? payload.progress_total
              : runsRef.current[taskEvent.run_id]?.progress_total,
          message: taskEvent.message,
          at: taskEvent.at,
        },
      }));
      setConnectionState("live");
    };

    source.addEventListener("open", handleOpen as EventListener);
    source.addEventListener("error", handleError as EventListener);
    source.addEventListener("snapshot", handleSnapshot as EventListener);
    source.addEventListener("run-upsert", handleRunUpsert as EventListener);
    source.addEventListener("run-event", handleRunEvent as EventListener);

    return () => {
      source.removeEventListener("open", handleOpen as EventListener);
      source.removeEventListener("error", handleError as EventListener);
      source.removeEventListener("snapshot", handleSnapshot as EventListener);
      source.removeEventListener("run-upsert", handleRunUpsert as EventListener);
      source.removeEventListener("run-event", handleRunEvent as EventListener);
      source.close();
    };
  }, [enabled, projectId]);

  const activeRunByNodeId = useMemo(() => {
    const active: Record<string, ProxyNodeLiveState> = {};
    for (const [nodeId, state] of Object.entries(nodeStateById)) {
      const run = runsById[state.runId];
      if (!run || isTerminal(run.status)) {
        continue;
      }
      active[nodeId] = {
        ...state,
        kind: run.kind as ProxyKind,
        progressCurrent: run.progress_current,
        progressTotal: run.progress_total,
      };
    }
    return active;
  }, [nodeStateById, runsById]);

  const runByNodeId = useMemo(() => {
    const latest: Record<string, ProxyNodeLiveState> = {};
    for (const [nodeId, state] of Object.entries(nodeStateById)) {
      const run = runsById[state.runId];
      latest[nodeId] = run
        ? {
            ...state,
            kind: run.kind as ProxyKind,
            progressCurrent: run.progress_current,
            progressTotal: run.progress_total,
          }
        : state;
    }
    return latest;
  }, [nodeStateById, runsById]);

  return {
    connectionState,
    runsById,
    nodeStateById,
    runByNodeId,
    activeRunByNodeId,
  };
}
