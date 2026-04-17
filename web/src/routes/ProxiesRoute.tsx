import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import { isGlobalProfileId } from "@/lib/profile-selection";
import type { LoadSubscriptionResponse, ProfileProxySettings, ProxyScope } from "@/lib/types";
import { ProxiesPage } from "@/pages/ProxiesPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function ProxiesRoute() {
  const { t } = useI18n();
  const queryClient = useQueryClient();
  const outlet = useOutletContext<RootOutletContext>();
  const { profileId, profiles, authMe, currentUser } = outlet;
  const isGlobalConfig = outlet.isGlobalConfig ?? isGlobalProfileId(profileId);
  const activeProfileId = outlet.activeProfileId ?? (isGlobalConfig ? null : profileId);
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
      queryClient.invalidateQueries({ queryKey: ["sessions"] }),
      queryClient.invalidateQueries({ queryKey: ["profiles"] }),
      requestedProfileId
        ? queryClient.invalidateQueries({
            queryKey: ["profile-proxy-settings", requestedProfileId],
          })
        : Promise.resolve(),
    ]);
  };

  const importQuery = useQuery({
    queryKey: ["proxy-imports"],
    queryFn: () => api.listProxyImports({ scope: "all" }),
    enabled: isGlobalConfig && canManageGlobal,
  });
  const profileProxySettingsQuery = useQuery({
    queryKey: ["profile-proxy-settings", activeProfileId],
    queryFn: () => api.getProfileProxySettings(activeProfileId ?? ""),
    enabled: Boolean(activeProfileId) && canManageGlobal,
  });

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
    />
  );
}
