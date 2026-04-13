import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { Navigate, useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import { isGlobalProfileId } from "@/lib/profile-selection";
import type { CreateApiKeyRequest, CreateApiKeyResponse, RefreshResponse } from "@/lib/types";
import { OverviewPage } from "@/pages/OverviewPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function OverviewRoute() {
  const { t } = useI18n();
  const outlet = useOutletContext<RootOutletContext>();
  const { profileId, profiles, authMe, currentUser } = outlet;
  const isGlobalConfig = outlet.isGlobalConfig ?? isGlobalProfileId(profileId);
  const activeProfileId = outlet.activeProfileId ?? (isGlobalConfig ? null : profileId);
  const previousProfileId = useRef(profileId);
  const queryClient = useQueryClient();
  const [refreshResponseByProfile, setRefreshResponseByProfile] = useState<
    Record<string, RefreshResponse | null>
  >({});
  const [latestCreatedApiKey, setLatestCreatedApiKey] = useState<CreateApiKeyResponse | null>(null);

  const healthQuery = useQuery({
    queryKey: ["health"],
    queryFn: api.getHealth,
    refetchInterval: 10_000,
  });
  const sessionsQuery = useQuery({
    queryKey: ["sessions", activeProfileId],
    queryFn: () => api.listSessions(activeProfileId ?? ""),
    enabled: Boolean(activeProfileId),
    refetchInterval: 5_000,
  });
  const apiKeysQuery = useQuery({
    queryKey: ["api-keys"],
    queryFn: api.listApiKeys,
    enabled: Boolean(authMe?.is_admin),
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
    mutationFn: (payload: CreateApiKeyRequest) => api.createApiKey(payload),
    onSuccess: async (data) => {
      setLatestCreatedApiKey(data);
      toast.success(t("Issued machine key {name}", { name: data.api_key.name }));
      await queryClient.invalidateQueries({ queryKey: ["api-keys"] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const revokeApiKeyMutation = useMutation({
    mutationFn: ({ keyId }: { keyId: string }) => api.revokeApiKey(keyId),
    onSuccess: async () => {
      toast.success(t("Revoked machine key"));
      await queryClient.invalidateQueries({ queryKey: ["api-keys"] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const { reset: resetRefreshMutation } = refreshMutation;

  useEffect(() => {
    if (previousProfileId.current === profileId) {
      return;
    }
    previousProfileId.current = profileId;
    resetRefreshMutation();
    setLatestCreatedApiKey(null);
  }, [profileId, resetRefreshMutation]);

  if (!activeProfileId) {
    return <Navigate replace to="/proxies" />;
  }

  return (
    <OverviewPage
      activeSessions={sessionsQuery.data?.sessions.length ?? 0}
      apiKeys={apiKeysQuery.data?.api_keys ?? []}
      apiKeysError={apiKeysQuery.isError ? formatApiErrorMessage(apiKeysQuery.error, t) : null}
      apiKeysLoading={apiKeysQuery.isLoading}
      availableProfiles={profiles}
      creatingApiKey={createApiKeyMutation.isPending}
      currentProfileId={profileId}
      currentUser={currentUser}
      health={healthQuery.data ?? { status: "checking" }}
      latestCreatedApiKey={latestCreatedApiKey}
      onCreateApiKey={async (payload) => {
        await createApiKeyMutation.mutateAsync(payload);
      }}
      onRefresh={async (payload) => {
        await refreshMutation.mutateAsync({ profileId: activeProfileId, payload });
      }}
      onRevokeApiKey={async (keyId) => {
        await revokeApiKeyMutation.mutateAsync({ keyId });
      }}
      refreshError={
        refreshMutation.isError ? formatApiErrorMessage(refreshMutation.error, t) : null
      }
      refreshResponse={refreshResponseByProfile[activeProfileId] ?? null}
      refreshing={refreshMutation.isPending}
      revokingApiKeyId={
        revokeApiKeyMutation.isPending ? (revokeApiKeyMutation.variables?.keyId ?? null) : null
      }
    />
  );
}
