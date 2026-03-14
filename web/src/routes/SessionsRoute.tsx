import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { ApiError, api } from "@/lib/api";
import { SessionsPage } from "@/pages/SessionsPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function SessionsRoute() {
  const {
    profileId,
    profileSummary,
    profileSummaryLoading,
    sessionsWorkspace,
    updateSessionsWorkspace,
  } = useOutletContext<RootOutletContext>();
  const queryClient = useQueryClient();
  const sessionsQuery = useQuery({
    queryKey: ["sessions", profileId],
    queryFn: () => api.listSessions(profileId),
    enabled: profileSummary?.initialized !== false,
    refetchInterval: 5_000,
  });

  const openMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.openSession>[1]) =>
      api.openSession(profileId, payload),
    onSuccess: async (data) => {
      updateSessionsWorkspace((current) => ({ ...current, openResponse: data }));
      toast.success(`Opened ${data.listen}`);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["sessions", profileId] }),
        queryClient.invalidateQueries({ queryKey: ["profile-summary", profileId] }),
      ]);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  const batchMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.openBatch>[1]) => api.openBatch(profileId, payload),
    onSuccess: async (data) => {
      updateSessionsWorkspace((current) => ({ ...current, batchResponse: data }));
      toast.success(`Opened ${data.sessions.length} sessions in batch`);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["sessions", profileId] }),
        queryClient.invalidateQueries({ queryKey: ["profile-summary", profileId] }),
      ]);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  const closeMutation = useMutation({
    mutationFn: (sessionId: string) => api.closeSession(profileId, sessionId),
    onSuccess: async (_, sessionId) => {
      toast.success(`Closed ${sessionId}`);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["sessions", profileId] }),
        queryClient.invalidateQueries({ queryKey: ["profile-summary", profileId] }),
      ]);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  return (
    <SessionsPage
      batchError={batchMutation.isError ? getErrorMessage(batchMutation.error) : null}
      batchFormValues={sessionsWorkspace.batchForm}
      batchOpening={batchMutation.isPending}
      batchResponse={sessionsWorkspace.batchResponse}
      closingSessionId={closeMutation.isPending ? closeMutation.variables : null}
      initialized={profileSummary?.initialized ?? false}
      initializationLoading={profileSummaryLoading}
      onBatchFormValuesChange={(values) =>
        updateSessionsWorkspace((current) => ({ ...current, batchForm: values }))
      }
      onCloseSession={async (sessionId) => {
        await closeMutation.mutateAsync(sessionId);
      }}
      onOpenBatch={async (payload) => {
        await batchMutation.mutateAsync(payload);
      }}
      onOpenSession={async (payload) => {
        await openMutation.mutateAsync(payload);
      }}
      onOpenFormValuesChange={(values) =>
        updateSessionsWorkspace((current) => ({ ...current, openForm: values }))
      }
      openError={openMutation.isError ? getErrorMessage(openMutation.error) : null}
      openFormValues={sessionsWorkspace.openForm}
      openResponse={sessionsWorkspace.openResponse}
      opening={openMutation.isPending}
      profileId={profileId}
      sessions={profileSummary?.initialized === false ? [] : (sessionsQuery.data?.sessions ?? [])}
      sessionsLoading={profileSummaryLoading || sessionsQuery.isLoading || sessionsQuery.isFetching}
    />
  );
}
