import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import type { LoadSubscriptionResponse, ProfileProxySettings, ProxyScope } from "@/lib/types";
import { ProxiesPage } from "@/pages/ProxiesPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function ProxiesRoute() {
  const { t } = useI18n();
  const queryClient = useQueryClient();
  const { profileId, profiles, authMe, currentUser } = useOutletContext<RootOutletContext>();
  const [globalLoadResponse, setGlobalLoadResponse] = useState<LoadSubscriptionResponse | null>(
    null,
  );
  const [profileLoadResponseByProfile, setProfileLoadResponseByProfile] = useState<
    Record<string, LoadSubscriptionResponse | null>
  >({});
  const canAccess =
    currentUser.status === "resolved" ? currentUser.identity.is_admin : Boolean(authMe?.is_admin);
  const accessDenied =
    currentUser.status === "anonymous" ||
    (currentUser.status === "resolved" && !currentUser.identity.is_admin);
  const authError = currentUser.status === "error" ? currentUser.message : null;

  const inventoryQuery = useQuery({
    queryKey: ["proxy-inventory"],
    queryFn: () => api.listProxyInventory({ scope: "all" }),
    enabled: canAccess,
  });
  const profileProxySettingsQuery = useQuery({
    queryKey: ["profile-proxy-settings", profileId],
    queryFn: () => api.getProfileProxySettings(profileId),
    enabled: canAccess,
  });

  const refreshProxyQueries = async (settingsProfileId: string) => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["proxy-inventory"] }),
      queryClient.invalidateQueries({ queryKey: ["profile-proxy-settings", settingsProfileId] }),
      queryClient.invalidateQueries({ queryKey: ["sessions"] }),
      queryClient.invalidateQueries({ queryKey: ["profiles"] }),
    ]);
  };

  const globalLoadMutation = useMutation({
    mutationFn: api.loadGlobalSubscription,
    onSuccess: async (response) => {
      setGlobalLoadResponse(response);
      toast.success(t("Imported {count} global proxies", { count: response.loaded_proxies }));
      await refreshProxyQueries(profileId);
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
    mutationFn: ({ nodeId, scope }: { nodeId: string; scope: ProxyScope }) =>
      api.updateProxyAllocation(nodeId, { allocation_scope: scope }),
    onSuccess: async (_, variables) => {
      toast.success(t("Updated allocation for {nodeId}", { nodeId: variables.nodeId }));
      await refreshProxyQueries(profileId);
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const deleteMutation = useMutation({
    mutationFn: (nodeId: string) => api.deleteProxyInventoryNode(nodeId),
    onSuccess: async (_, nodeId) => {
      toast.success(t("Deleted imported node {nodeId}", { nodeId }));
      await refreshProxyQueries(profileId);
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  return (
    <ProxiesPage
      accessDenied={accessDenied}
      authError={authError}
      currentUser={currentUser}
      deletingNodeId={deleteMutation.isPending ? (deleteMutation.variables ?? null) : null}
      globalLoadError={
        globalLoadMutation.isError ? formatApiErrorMessage(globalLoadMutation.error, t) : null
      }
      globalLoadResponse={globalLoadResponse}
      inventory={inventoryQuery.data ?? null}
      inventoryError={
        inventoryQuery.isError ? formatApiErrorMessage(inventoryQuery.error, t) : null
      }
      inventoryLoading={inventoryQuery.isLoading}
      loadingGlobal={globalLoadMutation.isPending}
      loadingProfile={profileLoadMutation.isPending}
      onDeleteNode={async (nodeId) => {
        await deleteMutation.mutateAsync(nodeId);
      }}
      onLoadGlobal={async (payload) => {
        await globalLoadMutation.mutateAsync(payload);
      }}
      onLoadProfile={async (payload) => {
        await profileLoadMutation.mutateAsync({ requestedProfileId: profileId, payload });
      }}
      onReassignNode={async (nodeId, scope) => {
        await reassignMutation.mutateAsync({ nodeId, scope });
      }}
      onToggleUseGlobalProxies={async (nextValue) => {
        await proxySettingsMutation.mutateAsync({
          requestedProfileId: profileId,
          useGlobalProxies: nextValue,
        });
      }}
      profileId={profileId}
      profileLoadError={
        profileLoadMutation.isError ? formatApiErrorMessage(profileLoadMutation.error, t) : null
      }
      profileLoadResponse={profileLoadResponseByProfile[profileId] ?? null}
      profiles={profiles}
      proxySettings={profileProxySettingsQuery.data ?? null}
      proxySettingsError={
        profileProxySettingsQuery.isError
          ? formatApiErrorMessage(profileProxySettingsQuery.error, t)
          : null
      }
      proxySettingsLoading={profileProxySettingsQuery.isLoading}
      reallocatingNodeId={
        reassignMutation.isPending ? (reassignMutation.variables?.nodeId ?? null) : null
      }
      updatingSettings={proxySettingsMutation.isPending}
    />
  );
}
