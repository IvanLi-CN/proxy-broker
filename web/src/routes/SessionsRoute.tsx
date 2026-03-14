import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { ApiError, api } from "@/lib/api";
import { SessionsPage } from "@/pages/SessionsPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function SessionsRoute() {
  const { profileId } = useOutletContext<RootOutletContext>();
  const queryClient = useQueryClient();
  const sessionsQuery = useQuery({
    queryKey: ["sessions", profileId],
    queryFn: () => api.listSessions(profileId),
    refetchInterval: 5_000,
  });

  const openMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.openSession>[1]) =>
      api.openSession(profileId, payload),
    onSuccess: async (data) => {
      toast.success(`Opened ${data.listen}`);
      await queryClient.invalidateQueries({ queryKey: ["sessions", profileId] });
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  const batchMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.openBatch>[1]) => api.openBatch(profileId, payload),
    onSuccess: async (data) => {
      toast.success(`Opened ${data.sessions.length} sessions in batch`);
      await queryClient.invalidateQueries({ queryKey: ["sessions", profileId] });
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  const closeMutation = useMutation({
    mutationFn: (sessionId: string) => api.closeSession(profileId, sessionId),
    onSuccess: async (_, sessionId) => {
      toast.success(`Closed ${sessionId}`);
      await queryClient.invalidateQueries({ queryKey: ["sessions", profileId] });
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  useEffect(() => {
    openMutation.reset();
    batchMutation.reset();
    closeMutation.reset();
  }, [profileId]);

  return (
    <SessionsPage
      batchError={batchMutation.isError ? getErrorMessage(batchMutation.error) : null}
      batchOpening={batchMutation.isPending}
      batchResponse={batchMutation.data ?? null}
      closingSessionId={closeMutation.isPending ? closeMutation.variables : null}
      onCloseSession={async (sessionId) => {
        await closeMutation.mutateAsync(sessionId);
      }}
      onOpenBatch={async (payload) => {
        await batchMutation.mutateAsync(payload);
      }}
      onOpenSession={async (payload) => {
        await openMutation.mutateAsync(payload);
      }}
      openError={openMutation.isError ? getErrorMessage(openMutation.error) : null}
      openResponse={openMutation.data ?? null}
      opening={openMutation.isPending}
      sessions={sessionsQuery.data?.sessions ?? []}
      sessionsLoading={sessionsQuery.isLoading}
    />
  );
}
