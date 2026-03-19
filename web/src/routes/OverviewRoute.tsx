import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { ApiError, api } from "@/lib/api";
import type { CreateApiKeyResponse, LoadSubscriptionResponse, RefreshResponse } from "@/lib/types";
import { OverviewPage } from "@/pages/OverviewPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function OverviewRoute() {
  const { profileId, authMe } = useOutletContext<RootOutletContext>();
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
      toast.success(`Loaded ${data.loaded_proxies} proxies for ${requestedProfileId}`);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
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
      toast.success(`Refreshed ${data.probed_ips} probe entries`);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  const createApiKeyMutation = useMutation({
    mutationFn: ({ profileId, name }: { profileId: string; name: string }) =>
      api.createApiKey(profileId, { name }),
    onSuccess: async (data, variables) => {
      setLatestApiKeyByProfile((current) => ({ ...current, [variables.profileId]: data }));
      toast.success(`Issued machine key ${data.api_key.name}`);
      await queryClient.invalidateQueries({ queryKey: ["api-keys", variables.profileId] });
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  const revokeApiKeyMutation = useMutation({
    mutationFn: ({ profileId, keyId }: { profileId: string; keyId: string }) =>
      api.revokeApiKey(profileId, keyId),
    onSuccess: async (_, variables) => {
      toast.success("Revoked machine key");
      await queryClient.invalidateQueries({ queryKey: ["api-keys", variables.profileId] });
    },
    onError: (error) => toast.error(getErrorMessage(error)),
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
      apiKeysError={apiKeysQuery.isError ? getErrorMessage(apiKeysQuery.error) : null}
      apiKeysLoading={apiKeysQuery.isLoading}
      creatingApiKey={createApiKeyMutation.isPending}
      health={healthQuery.data ?? { status: "checking" }}
      identity={authMe}
      loadError={loadMutation.isError ? getErrorMessage(loadMutation.error) : null}
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
      refreshError={refreshMutation.isError ? getErrorMessage(refreshMutation.error) : null}
      refreshResponse={refreshResponseByProfile[profileId] ?? null}
      refreshing={refreshMutation.isPending}
      revokingApiKeyId={
        revokeApiKeyMutation.isPending ? (revokeApiKeyMutation.variables?.keyId ?? null) : null
      }
    />
  );
}
