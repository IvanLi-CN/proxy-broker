import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useProxyOperationEvents } from "@/hooks/use-proxy-operation-events";
import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import { resolveSessionDisplayAddress } from "@/lib/format";
import { isGlobalProjectId } from "@/lib/project-selection";
import type {
  LoadSubscriptionResponse,
  ProjectProxySettings,
  ProxyOperationRequest,
  ProxyScope,
  SystemSettings,
  TaskRunSummary,
} from "@/lib/types";
import { ProxiesPage } from "@/pages/ProxiesPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const GLOBAL_TASK_PROJECT_ID = "__global__";

function isTerminalTask(run: TaskRunSummary) {
  return run.status === "succeeded" || run.status === "failed" || run.status === "skipped";
}

export function ProxiesRoute() {
  const { t } = useI18n();
  const queryClient = useQueryClient();
  const outlet = useOutletContext<RootOutletContext>();
  const { projectId, projects, authMe, currentUser } = outlet;
  const isGlobalProject = outlet.isGlobalProject ?? isGlobalProjectId(projectId);
  const activeProjectId = outlet.activeProjectId ?? (isGlobalProject ? null : projectId);
  const taskProjectId = isGlobalProject ? GLOBAL_TASK_PROJECT_ID : activeProjectId;
  const [globalLoadResponse, setGlobalLoadResponse] = useState<LoadSubscriptionResponse | null>(
    null,
  );
  const [projectLoadResponseByProject, setProjectLoadResponseByProject] = useState<
    Record<string, LoadSubscriptionResponse | null>
  >({});

  const canManageGlobal =
    currentUser.status === "resolved" ? currentUser.identity.is_admin : Boolean(authMe?.is_admin);
  const accessDenied =
    isGlobalProject &&
    (currentUser.status === "anonymous" ||
      (currentUser.status === "resolved" && !currentUser.identity.is_admin));
  const authError = isGlobalProject && currentUser.status === "error" ? currentUser.message : null;

  const refreshProxyQueries = async (requestedProjectId?: string | null) => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["proxy-imports"] }),
      queryClient.invalidateQueries({ queryKey: ["proxy-inventory"] }),
      queryClient.invalidateQueries({ queryKey: ["proxy-catalog"] }),
      queryClient.invalidateQueries({ queryKey: ["tasks"] }),
      queryClient.invalidateQueries({ queryKey: ["sessions"] }),
      queryClient.invalidateQueries({ queryKey: ["projects"] }),
      requestedProjectId
        ? queryClient.invalidateQueries({
            queryKey: ["project-proxy-settings", requestedProjectId],
          })
        : Promise.resolve(),
      requestedProjectId
        ? queryClient.invalidateQueries({
            queryKey: ["suggested-port", requestedProjectId],
          })
        : Promise.resolve(),
    ]);
  };

  const importQuery = useQuery({
    queryKey: ["proxy-imports"],
    queryFn: () => api.listProxyImports({ scope: "all" }),
    enabled: isGlobalProject && canManageGlobal,
  });
  const globalCatalogQuery = useQuery({
    queryKey: ["proxy-catalog", "global"],
    queryFn: () => api.listProxyCatalog({ view: "global" }),
    enabled: isGlobalProject && canManageGlobal,
  });
  const systemSettingsQuery = useQuery({
    queryKey: ["system-settings"],
    queryFn: api.getSystemSettings,
    enabled: isGlobalProject && canManageGlobal,
  });
  const projectCatalogQuery = useQuery({
    queryKey: ["proxy-catalog", "project", activeProjectId],
    queryFn: () => api.listProxyCatalog({ view: "project", project_id: activeProjectId ?? "" }),
    enabled: Boolean(activeProjectId),
  });
  const projectProxySettingsQuery = useQuery({
    queryKey: ["project-proxy-settings", activeProjectId],
    queryFn: () => api.getProjectProxySettings(activeProjectId ?? ""),
    enabled: Boolean(activeProjectId) && canManageGlobal,
  });
  const suggestedPortQuery = useQuery({
    queryKey: ["suggested-port", activeProjectId],
    queryFn: () => api.getSuggestedPort(activeProjectId ?? ""),
    enabled: Boolean(activeProjectId),
  });

  const proxyOperationEvents = useProxyOperationEvents({
    projectId: taskProjectId,
    enabled: Boolean(taskProjectId) && (!isGlobalProject || canManageGlobal),
  });
  const seenTerminalRunIds = useRef<Set<string>>(new Set());

  useEffect(() => {
    for (const run of Object.values(proxyOperationEvents.runsById)) {
      if (!isTerminalTask(run) || seenTerminalRunIds.current.has(run.run_id)) {
        continue;
      }
      seenTerminalRunIds.current.add(run.run_id);
      void queryClient.invalidateQueries({ queryKey: ["proxy-catalog"] });
    }
  }, [proxyOperationEvents.runsById, queryClient]);

  const globalLoadMutation = useMutation({
    mutationFn: api.loadGlobalSubscription,
    onSuccess: async (response) => {
      setGlobalLoadResponse(response);
      toast.success(t("Imported {count} global proxies", { count: response.loaded_proxies }));
      await refreshProxyQueries();
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });
  const projectLoadMutation = useMutation({
    mutationFn: ({
      requestedProjectId,
      payload,
    }: {
      requestedProjectId: string;
      payload: Parameters<typeof api.loadSubscription>[1];
    }) => api.loadSubscription(requestedProjectId, payload),
    onSuccess: async (response, { requestedProjectId }) => {
      setProjectLoadResponseByProject((current) => ({
        ...current,
        [requestedProjectId]: response,
      }));
      toast.success(
        t("Imported {count} project proxies for {projectId}", {
          count: response.loaded_proxies,
          projectId: requestedProjectId,
        }),
      );
      await refreshProxyQueries(requestedProjectId);
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });
  const proxySettingsMutation = useMutation({
    mutationFn: ({
      requestedProjectId,
      useGlobalProxies,
    }: {
      requestedProjectId: string;
      useGlobalProxies: boolean;
    }) =>
      api.updateProjectProxySettings(requestedProjectId, {
        use_global_proxies: useGlobalProxies,
      }),
    onSuccess: async (settings) => {
      queryClient.setQueryData<ProjectProxySettings>(
        ["project-proxy-settings", settings.project_id],
        settings,
      );
      toast.success(
        settings.use_global_proxies
          ? t("Enabled global pool for {projectId}", { projectId: settings.project_id })
          : t("Disabled global pool for {projectId}", { projectId: settings.project_id }),
      );
      await refreshProxyQueries(settings.project_id);
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });
  const systemSettingsMutation = useMutation({
    mutationFn: (proxyProbeIntervalSec: number) =>
      api.updateSystemSettings({ proxy_probe_interval_sec: proxyProbeIntervalSec }),
    onSuccess: (settings) => {
      queryClient.setQueryData<SystemSettings>(["system-settings"], settings);
      toast.success(
        t("Automatic latency probe interval saved: {minutes} minutes", {
          minutes: Math.round(settings.proxy_probe_interval_sec / 60),
        }),
      );
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });
  const reassignMutation = useMutation({
    mutationFn: ({ importId, scope }: { importId: string; scope: ProxyScope }) =>
      api.updateProxyImportAllocation(importId, { allocation_scope: scope }),
    onSuccess: async (_, variables) => {
      toast.success(t("Updated allocation for {importId}", { importId: variables.importId }));
      await refreshProxyQueries();
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });
  const deleteMutation = useMutation({
    mutationFn: (importId: string) => api.deleteProxyImport(importId),
    onSuccess: async (_, importId) => {
      toast.success(t("Deleted imported source {importId}", { importId }));
      await refreshProxyQueries();
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const proxyOperationMutation = useMutation({
    mutationFn: ({
      kind,
      payload,
    }: {
      kind: "refresh" | "probe";
      payload: ProxyOperationRequest;
    }) =>
      kind === "refresh"
        ? api.refreshProxyCatalogMetadata(payload)
        : api.probeProxyCatalogLatency(payload),
    onSuccess: (response, variables) => {
      toast.success(
        variables.kind === "refresh" ? t("Queued metadata refresh") : t("Queued latency probe"),
        {
          description: t("Run ID: {runId}", { runId: response.run_id }),
        },
      );
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const syncProxyImportsMutation = useMutation({
    mutationFn: (importIds: string[]) => api.syncProxyImports({ import_ids: importIds }),
    onSuccess: async (response) => {
      toast.success(t("Queued subscription update"), {
        description:
          response.run_ids.length > 0
            ? t("Run ID: {runId}", { runId: response.run_ids.join(", ") })
            : undefined,
      });
      await refreshProxyQueries(activeProjectId);
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const openSessionByNodeMutation = useMutation({
    mutationFn: ({
      requestedProjectId,
      payload,
    }: {
      requestedProjectId: string;
      payload: Parameters<typeof api.openSessionByNode>[1];
    }) => api.openSessionByNode(requestedProjectId, payload),
    onSuccess: async (response, { requestedProjectId }) => {
      toast.success(
        t("Listening on {listen} via {proxyName} ({selectedIp}).", {
          listen: resolveSessionDisplayAddress(response),
          proxyName: response.proxy_name,
          selectedIp: response.selected_ip,
        }),
      );
      await refreshProxyQueries(requestedProjectId);
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const openBatchByNodeMutation = useMutation({
    mutationFn: ({
      requestedProjectId,
      payload,
    }: {
      requestedProjectId: string;
      payload: Parameters<typeof api.openBatchByNode>[1];
    }) => api.openBatchByNode(requestedProjectId, payload),
    onSuccess: async (response, { requestedProjectId }) => {
      toast.success(t("Opened {count} sessions in batch", { count: response.sessions.length }));
      await refreshProxyQueries(requestedProjectId);
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  if (!isGlobalProject && activeProjectId) {
    return (
      <ProxiesPage
        mode="project"
        currentUser={currentUser}
        loadingProject={projectLoadMutation.isPending}
        onLoadProject={async (payload) => {
          await projectLoadMutation.mutateAsync({
            requestedProjectId: activeProjectId,
            payload,
          });
        }}
        onToggleUseGlobalProxies={async (nextValue) => {
          await proxySettingsMutation.mutateAsync({
            requestedProjectId: activeProjectId,
            useGlobalProxies: nextValue,
          });
        }}
        projectId={activeProjectId}
        suggestedPort={suggestedPortQuery.data?.port ?? null}
        projectLoadError={
          projectLoadMutation.isError ? formatApiErrorMessage(projectLoadMutation.error, t) : null
        }
        projectLoadResponse={projectLoadResponseByProject[activeProjectId] ?? null}
        proxySettings={projectProxySettingsQuery.data ?? null}
        proxySettingsError={
          projectProxySettingsQuery.isError
            ? formatApiErrorMessage(projectProxySettingsQuery.error, t)
            : null
        }
        proxySettingsLoading={projectProxySettingsQuery.isLoading}
        showProxyPolicy={canManageGlobal}
        updatingSettings={proxySettingsMutation.isPending}
        proxyCatalog={projectCatalogQuery.data ?? null}
        proxyCatalogLoading={projectCatalogQuery.isLoading}
        proxyCatalogError={
          projectCatalogQuery.isError ? formatApiErrorMessage(projectCatalogQuery.error, t) : null
        }
        liveConnectionState={proxyOperationEvents.connectionState}
        liveNodeStates={proxyOperationEvents.activeRunByNodeId}
        queueingOperation={proxyOperationMutation.isPending}
        onRefreshNodes={async (nodeIds) => {
          await proxyOperationMutation.mutateAsync({
            kind: "refresh",
            payload: { view: "project", project_id: activeProjectId, node_ids: nodeIds },
          });
        }}
        onProbeNodes={async (nodeIds) => {
          await proxyOperationMutation.mutateAsync({
            kind: "probe",
            payload: { view: "project", project_id: activeProjectId, node_ids: nodeIds },
          });
        }}
        onDeleteImport={async (importId) => {
          await deleteMutation.mutateAsync(importId);
        }}
        onSyncImports={async (importIds) => {
          await syncProxyImportsMutation.mutateAsync(importIds);
        }}
        onOpenSessionByNode={async (payload) => {
          await openSessionByNodeMutation.mutateAsync({
            requestedProjectId: activeProjectId,
            payload,
          });
        }}
        onOpenBatchByNode={async (payload) => {
          await openBatchByNodeMutation.mutateAsync({
            requestedProjectId: activeProjectId,
            payload,
          });
        }}
        deletingImportId={deleteMutation.isPending ? (deleteMutation.variables ?? null) : null}
        openingSessionNodeId={
          openSessionByNodeMutation.isPending
            ? (openSessionByNodeMutation.variables?.payload.node_id ?? null)
            : null
        }
        openingBatch={openBatchByNodeMutation.isPending}
        syncingImportIds={
          syncProxyImportsMutation.isPending ? (syncProxyImportsMutation.variables ?? []) : []
        }
      />
    );
  }

  return (
    <ProxiesPage
      mode="global"
      accessDenied={accessDenied}
      authError={authError}
      currentUser={currentUser}
      deletingImportId={deleteMutation.isPending ? (deleteMutation.variables ?? null) : null}
      globalLoadError={
        globalLoadMutation.isError ? formatApiErrorMessage(globalLoadMutation.error, t) : null
      }
      globalLoadResponse={globalLoadResponse}
      proxyImports={importQuery.data ?? null}
      proxyImportsError={importQuery.isError ? formatApiErrorMessage(importQuery.error, t) : null}
      proxyImportsLoading={importQuery.isLoading}
      systemSettings={systemSettingsQuery.data ?? null}
      systemSettingsLoading={systemSettingsQuery.isLoading}
      systemSettingsError={
        systemSettingsQuery.isError ? formatApiErrorMessage(systemSettingsQuery.error, t) : null
      }
      updatingSystemSettings={systemSettingsMutation.isPending}
      loadingGlobal={globalLoadMutation.isPending}
      onDeleteImport={async (importId) => {
        await deleteMutation.mutateAsync(importId);
      }}
      onLoadGlobal={async (payload) => {
        await globalLoadMutation.mutateAsync(payload);
      }}
      onUpdateSystemSettings={async (proxyProbeIntervalSec) => {
        await systemSettingsMutation.mutateAsync(proxyProbeIntervalSec);
      }}
      onReassignImport={async (importId, scope) => {
        await reassignMutation.mutateAsync({ importId, scope });
      }}
      onSyncImports={async (importIds) => {
        await syncProxyImportsMutation.mutateAsync(importIds);
      }}
      projects={projects}
      reallocatingImportId={
        reassignMutation.isPending ? (reassignMutation.variables?.importId ?? null) : null
      }
      proxyCatalog={globalCatalogQuery.data ?? null}
      proxyCatalogLoading={globalCatalogQuery.isLoading}
      proxyCatalogError={
        globalCatalogQuery.isError ? formatApiErrorMessage(globalCatalogQuery.error, t) : null
      }
      liveConnectionState={proxyOperationEvents.connectionState}
      liveNodeStates={proxyOperationEvents.activeRunByNodeId}
      queueingOperation={proxyOperationMutation.isPending}
      onRefreshNodes={async (nodeIds) => {
        await proxyOperationMutation.mutateAsync({
          kind: "refresh",
          payload: { view: "global", node_ids: nodeIds },
        });
      }}
      onProbeNodes={async (nodeIds) => {
        await proxyOperationMutation.mutateAsync({
          kind: "probe",
          payload: { view: "global", node_ids: nodeIds },
        });
      }}
      syncingImportIds={
        syncProxyImportsMutation.isPending ? (syncProxyImportsMutation.variables ?? []) : []
      }
    />
  );
}
