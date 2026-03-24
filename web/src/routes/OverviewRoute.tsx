import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import type { CreateApiKeyResponse, LoadSubscriptionResponse, RefreshResponse } from "@/lib/types";
import { OverviewPage } from "@/pages/OverviewPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function OverviewRoute() {
  const { t } = useI18n();
  const { profileId, authMe, currentUser } = useOutletContext<RootOutletContext>();
  const previousProfileId = useRef(profileId);
  const queryClient = useQueryClient();
  const [loadResponseByProfile, setLoadResponseByProfile] = useState<
    Record<string, LoadSubscriptionResponse | null>
  >({});
  const [refreshResponseByProfile, setRefreshResponseByProfile] = useState<
    Record<string, RefreshResponse | null>
  >({});
  const [latestApiKeyByProfile, setLatestApiKeyByProfile] = useState<
    Record<string, CreateApiKeyResponse | null>
  >({});
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

  const loadMutation = useMutation({
    mutationFn: ({
      profileId: requestedProfileId,
      payload,
    }: {
      profileId: string;
      payload: Parameters<typeof api.loadSubscription>[1];
    }) => api.loadSubscription(requestedProfileId, payload),
    onSuccess: (data, { profileId: requestedProfileId }) => {
      setLoadResponseByProfile((current) => ({ ...current, [requestedProfileId]: data }));
      toast.success(
        t("Loaded {count} proxies for {profileId}", {
          count: data.loaded_proxies,
          profileId: requestedProfileId,
        }),
      );
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

  const { reset: resetLoadMutation } = loadMutation;
  const { reset: resetRefreshMutation } = refreshMutation;

  useEffect(() => {
    if (previousProfileId.current === profileId) {
      return;
    }
    previousProfileId.current = profileId;
    resetLoadMutation();
    resetRefreshMutation();
  }, [profileId, resetLoadMutation, resetRefreshMutation]);

  return (
    <OverviewPage
      activeSessions={sessionsQuery.data?.sessions.length ?? 0}
      apiKeys={apiKeysQuery.data?.api_keys ?? []}
      apiKeysError={apiKeysQuery.isError ? formatApiErrorMessage(apiKeysQuery.error, t) : null}
      apiKeysLoading={apiKeysQuery.isLoading}
      creatingApiKey={createApiKeyMutation.isPending}
      currentUser={currentUser}
      health={healthQuery.data ?? { status: "checking" }}
      loadError={loadMutation.isError ? formatApiErrorMessage(loadMutation.error, t) : null}
      loadResponse={loadResponseByProfile[profileId] ?? null}
      loadingSubscription={loadMutation.isPending}
      latestCreatedApiKey={latestApiKeyByProfile[profileId] ?? null}
      onCreateApiKey={async (name) => {
        await createApiKeyMutation.mutateAsync({ profileId, name });
      }}
      onLoadSubscription={async (payload) => {
        await loadMutation.mutateAsync({ profileId, payload });
      }}
      onRefresh={async (payload) => {
        await refreshMutation.mutateAsync({ profileId, payload });
      }}
      onRevokeApiKey={async (keyId) => {
        await revokeApiKeyMutation.mutateAsync({ profileId, keyId });
      }}
      refreshError={
        refreshMutation.isError ? formatApiErrorMessage(refreshMutation.error, t) : null
      }
      refreshResponse={refreshResponseByProfile[profileId] ?? null}
      refreshing={refreshMutation.isPending}
      revokingApiKeyId={
        revokeApiKeyMutation.isPending ? (revokeApiKeyMutation.variables?.keyId ?? null) : null
      }
    />
  );
}
