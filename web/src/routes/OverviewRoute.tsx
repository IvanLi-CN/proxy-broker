import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { ApiError, api } from "@/lib/api";
import type { LoadSubscriptionResponse, RefreshResponse } from "@/lib/types";
import { OverviewPage } from "@/pages/OverviewPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function OverviewRoute() {
  const { profileId } = useOutletContext<RootOutletContext>();
  const [loadResponseByProfile, setLoadResponseByProfile] = useState<
    Record<string, LoadSubscriptionResponse | null>
  >({});
  const [refreshResponseByProfile, setRefreshResponseByProfile] = useState<
    Record<string, RefreshResponse | null>
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

  const loadMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.loadSubscription>[1]) =>
      api.loadSubscription(profileId, payload),
    onSuccess: (data) => {
      setLoadResponseByProfile((current) => ({ ...current, [profileId]: data }));
      toast.success(`Loaded ${data.loaded_proxies} proxies for ${profileId}`);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  const refreshMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.refreshProfile>[1]) =>
      api.refreshProfile(profileId, payload),
    onSuccess: (data) => {
      setRefreshResponseByProfile((current) => ({ ...current, [profileId]: data }));
      toast.success(`Refreshed ${data.probed_ips} probe entries`);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  useEffect(() => {
    loadMutation.reset();
    refreshMutation.reset();
  }, [profileId]);

  return (
    <OverviewPage
      activeSessions={sessionsQuery.data?.sessions.length ?? 0}
      health={healthQuery.data ?? { status: "checking" }}
      loadError={loadMutation.isError ? getErrorMessage(loadMutation.error) : null}
      loadResponse={loadResponseByProfile[profileId] ?? null}
      loadingSubscription={loadMutation.isPending}
      onLoadSubscription={async (payload) => {
        await loadMutation.mutateAsync(payload);
      }}
      onRefresh={async (payload) => {
        await refreshMutation.mutateAsync(payload);
      }}
      refreshError={refreshMutation.isError ? getErrorMessage(refreshMutation.error) : null}
      refreshResponse={refreshResponseByProfile[profileId] ?? null}
      refreshing={refreshMutation.isPending}
    />
  );
}
