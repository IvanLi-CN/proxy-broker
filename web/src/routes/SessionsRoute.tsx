import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { Navigate, useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useProxyOperationEvents } from "@/hooks/use-proxy-operation-events";
import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import { resolveSessionDisplayAddress } from "@/lib/format";
import { isGlobalProjectId } from "@/lib/project-selection";
import type { ProxyOperationRequest } from "@/lib/types";
import { SessionsPage } from "@/pages/SessionsPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const QUEUED_PROBE_NODE_LOCK_TTL_MS = 30_000;

export function SessionsRoute() {
  const { t } = useI18n();
  const outlet = useOutletContext<RootOutletContext>();
  const { projectId } = outlet;
  const isGlobalProject = outlet.isGlobalProject ?? isGlobalProjectId(projectId);
  const activeProjectId = outlet.activeProjectId ?? (isGlobalProject ? null : projectId);
  const previousProjectId = useRef(activeProjectId ?? "");
  const [queuedProbeNodeIdsByRun, setQueuedProbeNodeIdsByRun] = useState<
    Record<string, { nodeIds: string[]; queuedAt: number }>
  >({});
  const queryClient = useQueryClient();
  const sessionsQuery = useQuery({
    queryKey: ["sessions", activeProjectId],
    queryFn: () => api.listSessions(activeProjectId ?? ""),
    enabled: Boolean(activeProjectId),
    refetchInterval: 5_000,
  });
  const suggestedPortQuery = useQuery({
    queryKey: ["suggested-port", activeProjectId],
    queryFn: () => api.getSuggestedPort(activeProjectId ?? ""),
    enabled: Boolean(activeProjectId),
    refetchInterval: 5_000,
  });
  const proxyOperationEvents = useProxyOperationEvents({
    projectId: activeProjectId,
    enabled: Boolean(activeProjectId),
  });

  const openMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.openSessionByIp>[1]) =>
      api.openSessionByIp(activeProjectId ?? "", payload),
    onSuccess: async (data) => {
      toast.success(t("Opened {listen}", { listen: resolveSessionDisplayAddress(data) }));
      await queryClient.invalidateQueries({ queryKey: ["sessions", activeProjectId] });
      await queryClient.invalidateQueries({ queryKey: ["suggested-port", activeProjectId] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const batchMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.openBatchByIp>[1]) =>
      api.openBatchByIp(activeProjectId ?? "", payload),
    onSuccess: async (data) => {
      toast.success(t("Opened {count} sessions in batch", { count: data.sessions.length }));
      await queryClient.invalidateQueries({ queryKey: ["sessions", activeProjectId] });
      await queryClient.invalidateQueries({ queryKey: ["suggested-port", activeProjectId] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const switchMutation = useMutation({
    mutationFn: ({
      sessionId,
      payload,
    }: {
      sessionId: string;
      payload: Parameters<typeof api.updateSessionNode>[2];
    }) => api.updateSessionNode(activeProjectId ?? "", sessionId, payload),
    onSuccess: async (data, variables) => {
      toast.success(
        t("Switched {sessionId} to {proxyName}", {
          sessionId: variables.sessionId,
          proxyName: data.proxy_name,
        }),
      );
      await queryClient.invalidateQueries({ queryKey: ["sessions", activeProjectId] });
    },
  });

  const closeMutation = useMutation({
    mutationFn: (sessionId: string) => api.closeSession(activeProjectId ?? "", sessionId),
    onSuccess: async (_, sessionId) => {
      toast.success(t("Closed {sessionId}", { sessionId }));
      await queryClient.invalidateQueries({ queryKey: ["sessions", activeProjectId] });
      await queryClient.invalidateQueries({ queryKey: ["suggested-port", activeProjectId] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });
  const probeMutation = useMutation({
    mutationFn: (payload: ProxyOperationRequest) => api.probeProxyCatalogLatency(payload),
    onSuccess: (response, variables) => {
      setQueuedProbeNodeIdsByRun((current) => ({
        ...current,
        [response.run_id]: { nodeIds: variables.node_ids, queuedAt: Date.now() },
      }));
      toast.success(t("Queued latency probe"), {
        description: t("Run ID: {runId}", { runId: response.run_id }),
      });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const { reset: resetOpenMutation } = openMutation;
  const { reset: resetBatchMutation } = batchMutation;
  const { reset: resetSwitchMutation } = switchMutation;
  const { reset: resetCloseMutation } = closeMutation;
  const { reset: resetProbeMutation } = probeMutation;

  useEffect(() => {
    if (!activeProjectId) {
      return;
    }
    if (previousProjectId.current === activeProjectId) {
      return;
    }
    previousProjectId.current = activeProjectId;
    resetOpenMutation();
    resetBatchMutation();
    resetSwitchMutation();
    resetCloseMutation();
    resetProbeMutation();
    setQueuedProbeNodeIdsByRun({});
  }, [
    activeProjectId,
    resetBatchMutation,
    resetCloseMutation,
    resetOpenMutation,
    resetProbeMutation,
    resetSwitchMutation,
  ]);

  useEffect(() => {
    if (Object.keys(queuedProbeNodeIdsByRun).length === 0) {
      return undefined;
    }
    const pruneQueuedProbeNodes = () => {
      const now = Date.now();
      setQueuedProbeNodeIdsByRun((current) => {
        const next = Object.fromEntries(
          Object.entries(current).filter(([runId, queued]) => {
            const run = proxyOperationEvents.runsById[runId];
            if (run) {
              return !["succeeded", "failed", "skipped"].includes(run.status);
            }
            return now - queued.queuedAt < QUEUED_PROBE_NODE_LOCK_TTL_MS;
          }),
        );
        return Object.keys(next).length === Object.keys(current).length ? current : next;
      });
    };
    pruneQueuedProbeNodes();
    const intervalId = window.setInterval(pruneQueuedProbeNodes, 1_000);
    return () => window.clearInterval(intervalId);
  }, [proxyOperationEvents.runsById, queuedProbeNodeIdsByRun]);

  const queuedProbeNodeIds = useMemo(
    () => [
      ...new Set([
        ...(probeMutation.isPending ? (probeMutation.variables?.node_ids ?? []) : []),
        ...Object.values(queuedProbeNodeIdsByRun).flatMap((queued) => queued.nodeIds),
      ]),
    ],
    [probeMutation.isPending, probeMutation.variables, queuedProbeNodeIdsByRun],
  );

  if (!activeProjectId) {
    return <Navigate replace to="/proxies" />;
  }

  return (
    <SessionsPage
      batchError={batchMutation.isError ? formatApiErrorMessage(batchMutation.error, t) : null}
      batchOpening={batchMutation.isPending}
      batchResponse={batchMutation.data ?? null}
      closingSessionId={closeMutation.isPending ? closeMutation.variables : null}
      onCloseSession={async (sessionId) => {
        await closeMutation.mutateAsync(sessionId);
      }}
      onOpenBatch={async (payload) => {
        await batchMutation.mutateAsync(payload);
      }}
      onOpenSession={async (payload) => {
        await openMutation.mutateAsync(payload);
      }}
      onResetCreateState={() => {
        resetOpenMutation();
        resetBatchMutation();
      }}
      onResetSwitchState={() => {
        resetSwitchMutation();
      }}
      onUpdateSessionNode={async (sessionId, payload) => {
        await switchMutation.mutateAsync({ sessionId, payload });
      }}
      onProbeSessionNodes={async (nodeIds) => {
        await probeMutation.mutateAsync({
          view: "project",
          project_id: activeProjectId,
          node_ids: nodeIds,
        });
      }}
      openError={openMutation.isError ? formatApiErrorMessage(openMutation.error, t) : null}
      openResponse={openMutation.data ?? null}
      opening={openMutation.isPending}
      searchSessionIpNodeOptions={async (payload) =>
        (await api.searchSessionIpNodeOptions(activeProjectId, payload)).groups
      }
      searchSessionNodeOptions={async (sessionId, payload) =>
        (await api.searchSessionNodeOptions(activeProjectId, sessionId, payload)).items
      }
      sessions={sessionsQuery.data?.sessions ?? []}
      sessionsLoading={sessionsQuery.isLoading}
      suggestedPort={suggestedPortQuery.data?.port ?? null}
      switchError={switchMutation.isError ? formatApiErrorMessage(switchMutation.error, t) : null}
      switchedSessionId={switchMutation.data?.session_id ?? null}
      probingNodeIds={queuedProbeNodeIds}
      liveNodeStates={proxyOperationEvents.activeRunByNodeId}
      probeNodeStates={proxyOperationEvents.runByNodeId}
      switchingSessionId={
        switchMutation.isPending ? (switchMutation.variables?.sessionId ?? null) : null
      }
    />
  );
}
