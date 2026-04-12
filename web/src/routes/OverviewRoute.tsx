import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import type {
  CreateApiKeyResponse,
  LoadSubscriptionResponse,
  ProfileProxySettings,
  RefreshResponse,
} from "@/lib/types";
import { OverviewPage } from "@/pages/OverviewPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function OverviewRoute() {
  const { t } = useI18n();
  const { profileId, authMe, currentUser } = useOutletContext<RootOutletContext>();
  const queryClient = useQueryClient();
  const [profileLoadResponseByProfile, setProfileLoadResponseByProfile] = useState<
    Record<string, LoadSubscriptionResponse | null>
  >({});
  const [refreshResponseByProfile, setRefreshResponseByProfile] = useState<
    Record<string, RefreshResponse | null>
  >({});
  const [latestApiKeyByProfile, setLatestApiKeyByProfile] = useState<
    Record<string, CreateApiKeyResponse | null>
  >({});
  const canManageProxyPolicy =
    currentUser.status === "resolved" ? currentUser.identity.is_admin : Boolean(authMe?.is_admin);
  const healthQuery = useQuery({
    queryKey: ["health"],
    queryFn: api.getHealth,
    refetchInterval: 10_000,
  });
  const sessionsQuery = useQuery({
    queryKey: ["sessions", profileId],
    queryFn: () => api.listSessions(profileId),
    refetchInterval: 5_000,
  });
  const apiKeysQuery = useQuery({
    queryKey: ["api-keys", profileId],
    queryFn: () => api.listApiKeys(profileId),
    enabled: Boolean(authMe?.is_admin),
  });
  const profileProxySettingsQuery = useQuery({
    queryKey: ["profile-proxy-settings", profileId],
    queryFn: () => api.getProfileProxySettings(profileId),
    enabled: canManageProxyPolicy,
  });

  const refreshProxyQueries = async (requestedProfileId: string) => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["proxy-inventory"] }),
      queryClient.invalidateQueries({ queryKey: ["profile-proxy-settings", requestedProfileId] }),
      queryClient.invalidateQueries({ queryKey: ["sessions"] }),
      queryClient.invalidateQueries({ queryKey: ["profiles"] }),
    ]);
  };

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

  const refreshMutation = useMutation({
    mutationFn: ({
      profileId: requestedProfileId,
      payload,
    }: {
      profileId: string;
      payload: Parameters<typeof api.refreshProfile>[1];
    }) => api.refreshProfile(requestedProfileId, payload),
    onSuccess: (data, { profileId: requestedProfileId }) => {
      setRefreshResponseByProfile((current) => ({ ...current, [requestedProfileId]: data }));
      toast.success(t("Refreshed {count} probe entries", { count: data.probed_ips }));
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

  const createApiKeyMutation = useMutation({
    mutationFn: ({ profileId, name }: { profileId: string; name: string }) =>
      api.createApiKey(profileId, { name }),
    onSuccess: async (data, variables) => {
      setLatestApiKeyByProfile((current) => ({ ...current, [variables.profileId]: data }));
      toast.success(t("Issued machine key {name}", { name: data.api_key.name }));
      await queryClient.invalidateQueries({ queryKey: ["api-keys", variables.profileId] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const revokeApiKeyMutation = useMutation({
    mutationFn: ({ profileId, keyId }: { profileId: string; keyId: string }) =>
      api.revokeApiKey(profileId, keyId),
    onSuccess: async (_, variables) => {
      toast.success(t("Revoked machine key"));
      await queryClient.invalidateQueries({ queryKey: ["api-keys", variables.profileId] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  return (
    <OverviewPage
      activeSessions={sessionsQuery.data?.sessions.length ?? 0}
      apiKeys={apiKeysQuery.data?.api_keys ?? []}
      apiKeysError={apiKeysQuery.isError ? formatApiErrorMessage(apiKeysQuery.error, t) : null}
      apiKeysLoading={apiKeysQuery.isLoading}
      creatingApiKey={createApiKeyMutation.isPending}
      currentUser={currentUser}
      health={healthQuery.data ?? { status: "checking" }}
      latestCreatedApiKey={latestApiKeyByProfile[profileId] ?? null}
      loadingProfile={profileLoadMutation.isPending}
      onLoadProfile={async (payload) => {
        await profileLoadMutation.mutateAsync({ requestedProfileId: profileId, payload });
      }}
      onCreateApiKey={async (name) => {
        await createApiKeyMutation.mutateAsync({ profileId, name });
      }}
      onRefresh={async (payload) => {
        await refreshMutation.mutateAsync({ profileId, payload });
      }}
      onRevokeApiKey={async (keyId) => {
        await revokeApiKeyMutation.mutateAsync({ profileId, keyId });
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
      proxySettings={profileProxySettingsQuery.data ?? null}
      proxySettingsError={
        profileProxySettingsQuery.isError
          ? formatApiErrorMessage(profileProxySettingsQuery.error, t)
          : null
      }
      proxySettingsLoading={profileProxySettingsQuery.isLoading}
      refreshError={
        refreshMutation.isError ? formatApiErrorMessage(refreshMutation.error, t) : null
      }
      refreshResponse={refreshResponseByProfile[profileId] ?? null}
      refreshing={refreshMutation.isPending}
      revokingApiKeyId={
        revokeApiKeyMutation.isPending ? (revokeApiKeyMutation.variables?.keyId ?? null) : null
      }
      showProxyPolicy={canManageProxyPolicy}
      updatingSettings={proxySettingsMutation.isPending}
    />
  );
}
