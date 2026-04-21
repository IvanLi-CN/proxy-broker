import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useProxyOperationEvents } from "@/hooks/use-proxy-operation-events";
import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import { isGlobalProfileId } from "@/lib/profile-selection";
import type {
  LoadSubscriptionResponse,
  ProfileProxySettings,
  ProxyOperationRequest,
  ProxyScope,
  TaskRunSummary,
} from "@/lib/types";
import { ProxiesPage } from "@/pages/ProxiesPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const GLOBAL_TASK_PROFILE_ID = "__global__";

function isTerminalTask(run: TaskRunSummary) {
  return run.status === "succeeded" || run.status === "failed" || run.status === "skipped";
}

export function ProxiesRoute() {
  const { t } = useI18n();
  const queryClient = useQueryClient();
  const outlet = useOutletContext<RootOutletContext>();
  const { profileId, profiles, authMe, currentUser } = outlet;
  const isGlobalConfig = outlet.isGlobalConfig ?? isGlobalProfileId(profileId);
  const activeProfileId = outlet.activeProfileId ?? (isGlobalConfig ? null : profileId);
  const taskProfileId = isGlobalConfig ? GLOBAL_TASK_PROFILE_ID : activeProfileId;
  const [globalLoadResponse, setGlobalLoadResponse] = useState<LoadSubscriptionResponse | null>(
    null,
  );
  const [profileLoadResponseByProfile, setProfileLoadResponseByProfile] = useState<
    Record<string, LoadSubscriptionResponse | null>
  >({});

  const canManageGlobal =
    currentUser.status === "resolved" ? currentUser.identity.is_admin : Boolean(authMe?.is_admin);
  const accessDenied =
    isGlobalConfig &&
    (currentUser.status === "anonymous" ||
      (currentUser.status === "resolved" && !currentUser.identity.is_admin));
  const authError = isGlobalConfig && currentUser.status === "error" ? currentUser.message : null;

  const refreshProxyQueries = async (requestedProfileId?: string | null) => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["proxy-imports"] }),
      queryClient.invalidateQueries({ queryKey: ["proxy-inventory"] }),
      queryClient.invalidateQueries({ queryKey: ["proxy-catalog"] }),
      queryClient.invalidateQueries({ queryKey: ["sessions"] }),
      queryClient.invalidateQueries({ queryKey: ["profiles"] }),
      requestedProfileId
        ? queryClient.invalidateQueries({
            queryKey: ["profile-proxy-settings", requestedProfileId],
          })
        : Promise.resolve(),
      requestedProfileId
        ? queryClient.invalidateQueries({
            queryKey: ["suggested-port", requestedProfileId],
          })
        : Promise.resolve(),
    ]);
  };

  const importQuery = useQuery({
    queryKey: ["proxy-imports"],
    queryFn: () => api.listProxyImports({ scope: "all" }),
    enabled: isGlobalConfig && canManageGlobal,
  });
  const globalCatalogQuery = useQuery({
    queryKey: ["proxy-catalog", "global"],
    queryFn: () => api.listProxyCatalog({ view: "global" }),
    enabled: isGlobalConfig && canManageGlobal,
  });
  const profileCatalogQuery = useQuery({
    queryKey: ["proxy-catalog", "profile", activeProfileId],
    queryFn: () => api.listProxyCatalog({ view: "profile", profile_id: activeProfileId ?? "" }),
    enabled: Boolean(activeProfileId) && canManageGlobal,
  });
  const profileProxySettingsQuery = useQuery({
    queryKey: ["profile-proxy-settings", activeProfileId],
    queryFn: () => api.getProfileProxySettings(activeProfileId ?? ""),
    enabled: Boolean(activeProfileId) && canManageGlobal,
  });
  const suggestedPortQuery = useQuery({
    queryKey: ["suggested-port", activeProfileId],
    queryFn: () => api.getSuggestedPort(activeProfileId ?? ""),
    enabled: Boolean(activeProfileId) && canManageGlobal,
  });

  const proxyOperationEvents = useProxyOperationEvents({
    profileId: taskProfileId,
    enabled: Boolean(taskProfileId) && (!isGlobalConfig || canManageGlobal),
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
  const profileLoadMutation = useMutation({
    mutationFn: ({
      requestedProfileId,
      payload,
    }: {
      requestedProfileId: string;
      payload: Parameters<typeof api.loadSubscription>[1];
    }) => api.loadSubscription(requestedProfileId, payload),
    onSuccess: async (response, { requestedProfileId }) => {
      setProfileLoadResponseByProfile((current) => ({
        ...current,
        [requestedProfileId]: response,
      }));
      toast.success(
        t("Imported {count} profile proxies for {profileId}", {
          count: response.loaded_proxies,
          profileId: requestedProfileId,
        }),
      );
      await refreshProxyQueries(requestedProfileId);
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });
  const proxySettingsMutation = useMutation({
    mutationFn: ({
      requestedProfileId,
      useGlobalProxies,
    }: {
      requestedProfileId: string;
      useGlobalProxies: boolean;
    }) =>
      api.updateProfileProxySettings(requestedProfileId, {
        use_global_proxies: useGlobalProxies,
      }),
    onSuccess: async (settings) => {
      queryClient.setQueryData<ProfileProxySettings>(
        ["profile-proxy-settings", settings.profile_id],
        settings,
      );
      toast.success(
        settings.use_global_proxies
          ? t("Enabled global pool for {profileId}", { profileId: settings.profile_id })
          : t("Disabled global pool for {profileId}", { profileId: settings.profile_id }),
      );
      await refreshProxyQueries(settings.profile_id);
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

  const openSessionByNodeMutation = useMutation({
    mutationFn: ({
      requestedProfileId,
      payload,
    }: {
      requestedProfileId: string;
      payload: Parameters<typeof api.openSessionByNode>[1];
    }) => api.openSessionByNode(requestedProfileId, payload),
    onSuccess: async (response, { requestedProfileId }) => {
      toast.success(
        t("Listening on {listen} via {proxyName} ({selectedIp}).", {
          listen: `${response.listen}:${response.port}`,
          proxyName: response.proxy_name,
          selectedIp: response.selected_ip,
        }),
      );
      await refreshProxyQueries(requestedProfileId);
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const openBatchByNodeMutation = useMutation({
    mutationFn: ({
      requestedProfileId,
      payload,
    }: {
      requestedProfileId: string;
      payload: Parameters<typeof api.openBatchByNode>[1];
    }) => api.openBatchByNode(requestedProfileId, payload),
    onSuccess: async (response, { requestedProfileId }) => {
      toast.success(t("Opened {count} sessions in batch", { count: response.sessions.length }));
      await refreshProxyQueries(requestedProfileId);
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  if (!isGlobalConfig && activeProfileId) {
    return (
      <ProxiesPage
        mode="profile"
        currentUser={currentUser}
        loadingProfile={profileLoadMutation.isPending}
        onLoadProfile={async (payload) => {
          await profileLoadMutation.mutateAsync({
            requestedProfileId: activeProfileId,
            payload,
          });
        }}
        onToggleUseGlobalProxies={async (nextValue) => {
          await proxySettingsMutation.mutateAsync({
            requestedProfileId: activeProfileId,
            useGlobalProxies: nextValue,
          });
        }}
        profileId={activeProfileId}
        suggestedPort={suggestedPortQuery.data?.port ?? null}
        profileLoadError={
          profileLoadMutation.isError ? formatApiErrorMessage(profileLoadMutation.error, t) : null
        }
        profileLoadResponse={profileLoadResponseByProfile[activeProfileId] ?? null}
        proxySettings={profileProxySettingsQuery.data ?? null}
        proxySettingsError={
          profileProxySettingsQuery.isError
            ? formatApiErrorMessage(profileProxySettingsQuery.error, t)
            : null
        }
        proxySettingsLoading={profileProxySettingsQuery.isLoading}
        showProxyPolicy={canManageGlobal}
        updatingSettings={proxySettingsMutation.isPending}
        proxyCatalog={profileCatalogQuery.data ?? null}
        proxyCatalogLoading={profileCatalogQuery.isLoading}
        proxyCatalogError={
          profileCatalogQuery.isError ? formatApiErrorMessage(profileCatalogQuery.error, t) : null
        }
        liveConnectionState={proxyOperationEvents.connectionState}
        liveNodeStates={proxyOperationEvents.activeRunByNodeId}
        queueingOperation={proxyOperationMutation.isPending}
        onRefreshNodes={async (nodeIds) => {
          await proxyOperationMutation.mutateAsync({
            kind: "refresh",
            payload: { view: "profile", profile_id: activeProfileId, node_ids: nodeIds },
          });
        }}
        onProbeNodes={async (nodeIds) => {
          await proxyOperationMutation.mutateAsync({
            kind: "probe",
            payload: { view: "profile", profile_id: activeProfileId, node_ids: nodeIds },
          });
        }}
        onDeleteImport={async (importId) => {
          await deleteMutation.mutateAsync(importId);
        }}
        onOpenSessionByNode={async (payload) => {
          await openSessionByNodeMutation.mutateAsync({
            requestedProfileId: activeProfileId,
            payload,
          });
        }}
        onOpenBatchByNode={async (payload) => {
          await openBatchByNodeMutation.mutateAsync({
            requestedProfileId: activeProfileId,
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
      loadingGlobal={globalLoadMutation.isPending}
      onDeleteImport={async (importId) => {
        await deleteMutation.mutateAsync(importId);
      }}
      onLoadGlobal={async (payload) => {
        await globalLoadMutation.mutateAsync(payload);
      }}
      onReassignImport={async (importId, scope) => {
        await reassignMutation.mutateAsync({ importId, scope });
      }}
      profiles={profiles}
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
    />
  );
}
