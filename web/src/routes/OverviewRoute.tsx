import { useMutation, useQuery } from "@tanstack/react-query";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { ApiError, api } from "@/lib/api";
import { OverviewPage } from "@/pages/OverviewPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function OverviewRoute() {
  const { profileId } = useOutletContext<RootOutletContext>();
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

  const loadMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.loadSubscription>[1]) =>
      api.loadSubscription(profileId, payload),
    onSuccess: (data) => {
      toast.success(`Loaded ${data.loaded_proxies} proxies for ${profileId}`);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  const refreshMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.refreshProfile>[1]) =>
      api.refreshProfile(profileId, payload),
    onSuccess: (data) => {
      toast.success(`Refreshed ${data.probed_ips} probe entries`);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  return (
    <OverviewPage
      activeSessions={sessionsQuery.data?.sessions.length ?? 0}
      health={healthQuery.data ?? { status: "checking" }}
      loadError={loadMutation.isError ? getErrorMessage(loadMutation.error) : null}
      loadResponse={loadMutation.data ?? null}
      loadingSubscription={loadMutation.isPending}
      onLoadSubscription={async (payload) => {
        await loadMutation.mutateAsync(payload);
      }}
      onRefresh={async (payload) => {
        await refreshMutation.mutateAsync(payload);
      }}
      refreshError={refreshMutation.isError ? getErrorMessage(refreshMutation.error) : null}
      refreshResponse={refreshMutation.data ?? null}
      refreshing={refreshMutation.isPending}
    />
  );
}
