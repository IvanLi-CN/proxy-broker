import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { Navigate, useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import { isGlobalProjectId } from "@/lib/project-selection";
import type { CreateApiKeyRequest, CreateApiKeyResponse, RefreshResponse } from "@/lib/types";
import { OverviewPage } from "@/pages/OverviewPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function OverviewRoute() {
  const { t } = useI18n();
  const outlet = useOutletContext<RootOutletContext>();
  const { projectId, projects, authMe, currentUser } = outlet;
  const isGlobalProject = outlet.isGlobalProject ?? isGlobalProjectId(projectId);
  const activeProjectId = outlet.activeProjectId ?? (isGlobalProject ? null : projectId);
  const previousProjectId = useRef(projectId);
  const previousApiKeyOwnerSubject = useRef<string | null>(null);
  const queryClient = useQueryClient();
  const apiKeyOwnerSubject = authMe?.subject ?? null;
  const [refreshResponseByProject, setRefreshResponseByProject] = useState<
    Record<string, RefreshResponse | null>
  >({});
  const [latestCreatedApiKey, setLatestCreatedApiKey] = useState<CreateApiKeyResponse | null>(null);

  const healthQuery = useQuery({
    queryKey: ["health"],
    queryFn: api.getHealth,
    refetchInterval: 10_000,
  });
  const sessionsQuery = useQuery({
    queryKey: ["sessions", activeProjectId],
    queryFn: () => api.listSessions(activeProjectId ?? ""),
    enabled: Boolean(activeProjectId),
    refetchInterval: 5_000,
  });
  const apiKeysQuery = useQuery({
    queryKey: ["api-keys", apiKeyOwnerSubject],
    queryFn: api.listApiKeys,
    enabled: Boolean(authMe?.is_admin),
  });

  const refreshMutation = useMutation({
    mutationFn: ({
      projectId: requestedProjectId,
      payload,
    }: {
      projectId: string;
      payload: Parameters<typeof api.refreshProject>[1];
    }) => api.refreshProject(requestedProjectId, payload),
    onSuccess: (data, { projectId: requestedProjectId }) => {
      setRefreshResponseByProject((current) => ({ ...current, [requestedProjectId]: data }));
      toast.success(t("Refreshed {count} probe entries", { count: data.probed_ips }));
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const createApiKeyMutation = useMutation({
    mutationFn: (payload: CreateApiKeyRequest) => api.createApiKey(payload),
    onSuccess: async (data) => {
      setLatestCreatedApiKey(data);
      toast.success(t("Issued machine key {name}", { name: data.api_key.name }));
      await queryClient.invalidateQueries({ queryKey: ["api-keys", apiKeyOwnerSubject] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const revokeApiKeyMutation = useMutation({
    mutationFn: ({ keyId }: { keyId: string }) => api.revokeApiKey(keyId),
    onSuccess: async () => {
      toast.success(t("Revoked machine key"));
      await queryClient.invalidateQueries({ queryKey: ["api-keys", apiKeyOwnerSubject] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const { reset: resetRefreshMutation } = refreshMutation;

  useEffect(() => {
    if (previousProjectId.current === projectId) {
      return;
    }
    previousProjectId.current = projectId;
    resetRefreshMutation();
    setLatestCreatedApiKey(null);
  }, [projectId, resetRefreshMutation]);

  useEffect(() => {
    if (previousApiKeyOwnerSubject.current === apiKeyOwnerSubject) {
      return;
    }
    previousApiKeyOwnerSubject.current = apiKeyOwnerSubject;
    setLatestCreatedApiKey(null);
  }, [apiKeyOwnerSubject]);

  if (!activeProjectId) {
    return <Navigate replace to="/proxies" />;
  }

  return (
    <OverviewPage
      activeSessions={sessionsQuery.data?.sessions.length ?? 0}
      apiKeys={apiKeysQuery.data?.api_keys ?? []}
      apiKeysError={apiKeysQuery.isError ? formatApiErrorMessage(apiKeysQuery.error, t) : null}
      apiKeysLoading={apiKeysQuery.isLoading}
      availableProjects={projects}
      creatingApiKey={createApiKeyMutation.isPending}
      currentProjectId={projectId}
      currentUser={currentUser}
      health={healthQuery.data ?? { status: "checking" }}
      latestCreatedApiKey={latestCreatedApiKey}
      onCreateApiKey={async (payload) => {
        await createApiKeyMutation.mutateAsync(payload);
      }}
      onRefresh={async (payload) => {
        await refreshMutation.mutateAsync({ projectId: activeProjectId, payload });
      }}
      onRevokeApiKey={async (keyId) => {
        await revokeApiKeyMutation.mutateAsync({ keyId });
      }}
      refreshError={
        refreshMutation.isError ? formatApiErrorMessage(refreshMutation.error, t) : null
      }
      refreshResponse={refreshResponseByProject[activeProjectId] ?? null}
      refreshing={refreshMutation.isPending}
      revokingApiKeyId={
        revokeApiKeyMutation.isPending ? (revokeApiKeyMutation.variables?.keyId ?? null) : null
      }
    />
  );
}
