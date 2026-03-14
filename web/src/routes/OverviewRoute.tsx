import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { ApiError, api } from "@/lib/api";
import { OverviewPage } from "@/pages/OverviewPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function OverviewRoute() {
  const {
    profileId,
    profileSummary,
    profileSummaryLoading,
    overviewWorkspace,
    updateOverviewWorkspace,
  } = useOutletContext<RootOutletContext>();
  const queryClient = useQueryClient();
  const healthQuery = useQuery({
    queryKey: ["health"],
    queryFn: api.getHealth,
    refetchInterval: 10_000,
  });

  const loadMutation = useMutation({
    mutationFn: ({
      profileId: requestedProfileId,
      payload,
    }: {
      profileId: string;
      payload: Parameters<typeof api.loadSubscription>[1];
    }) => api.loadSubscription(requestedProfileId, payload),
    onSuccess: async (data, { profileId: requestedProfileId }) => {
      updateOverviewWorkspace((current) => ({ ...current, loadResponse: data }));
      toast.success(`Loaded ${data.loaded_proxies} proxies for ${requestedProfileId}`);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["profiles"] }),
        queryClient.invalidateQueries({ queryKey: ["profile-summary", requestedProfileId] }),
      ]);
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
    onSuccess: async (data, { profileId: requestedProfileId }) => {
      updateOverviewWorkspace((current) => ({ ...current, refreshResponse: data }));
      toast.success(`Refreshed ${data.probed_ips} probe entries`);
      await queryClient.invalidateQueries({ queryKey: ["profile-summary", requestedProfileId] });
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  return (
    <OverviewPage
      activeSessions={profileSummary?.session_count ?? 0}
      health={healthQuery.data ?? { status: "checking" }}
      initialized={profileSummary?.initialized ?? false}
      initializationLoading={profileSummaryLoading}
      loadError={loadMutation.isError ? getErrorMessage(loadMutation.error) : null}
      loadResponse={overviewWorkspace.loadResponse}
      loadingSubscription={loadMutation.isPending}
      onLoadSubscription={async (payload) => {
        await loadMutation.mutateAsync({ profileId, payload });
      }}
      onRefresh={async (payload) => {
        await refreshMutation.mutateAsync({ profileId, payload });
      }}
      poolInventory={
        profileSummary?.proxy_count ?? overviewWorkspace.loadResponse?.loaded_proxies ?? null
      }
      profileId={profileId}
      refreshError={refreshMutation.isError ? getErrorMessage(refreshMutation.error) : null}
      refreshResponse={overviewWorkspace.refreshResponse}
      refreshing={refreshMutation.isPending}
      subscriptionFormValues={overviewWorkspace.subscriptionForm}
      onSubscriptionFormValuesChange={(values) =>
        updateOverviewWorkspace((current) => ({ ...current, subscriptionForm: values }))
      }
      refreshFormValues={overviewWorkspace.refreshForm}
      onRefreshFormValuesChange={(values) =>
        updateOverviewWorkspace((current) => ({ ...current, refreshForm: values }))
      }
    />
  );
}
