import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import { SessionsPage } from "@/pages/SessionsPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function SessionsRoute() {
  const { t } = useI18n();
  const { profileId } = useOutletContext<RootOutletContext>();
  const previousProfileId = useRef(profileId);
  const queryClient = useQueryClient();
  const sessionsQuery = useQuery({
    queryKey: ["sessions", profileId],
    queryFn: () => api.listSessions(profileId),
    refetchInterval: 5_000,
  });
  const suggestedPortQuery = useQuery({
    queryKey: ["suggested-port", profileId],
    queryFn: () => api.getSuggestedPort(profileId),
    refetchInterval: 5_000,
  });

  const openMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.openSession>[1]) =>
      api.openSession(profileId, payload),
    onSuccess: async (data) => {
      toast.success(t("Opened {listen}", { listen: data.listen }));
      await queryClient.invalidateQueries({ queryKey: ["sessions", profileId] });
      await queryClient.invalidateQueries({ queryKey: ["suggested-port", profileId] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const batchMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.openBatch>[1]) => api.openBatch(profileId, payload),
    onSuccess: async (data) => {
      toast.success(t("Opened {count} sessions in batch", { count: data.sessions.length }));
      await queryClient.invalidateQueries({ queryKey: ["sessions", profileId] });
      await queryClient.invalidateQueries({ queryKey: ["suggested-port", profileId] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const closeMutation = useMutation({
    mutationFn: (sessionId: string) => api.closeSession(profileId, sessionId),
    onSuccess: async (_, sessionId) => {
      toast.success(t("Closed {sessionId}", { sessionId }));
      await queryClient.invalidateQueries({ queryKey: ["sessions", profileId] });
      await queryClient.invalidateQueries({ queryKey: ["suggested-port", profileId] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const { reset: resetOpenMutation } = openMutation;
  const { reset: resetBatchMutation } = batchMutation;
  const { reset: resetCloseMutation } = closeMutation;

  useEffect(() => {
    if (previousProfileId.current === profileId) {
      return;
    }
    previousProfileId.current = profileId;
    resetOpenMutation();
    resetBatchMutation();
    resetCloseMutation();
  }, [profileId, resetOpenMutation, resetBatchMutation, resetCloseMutation]);

  return (
    <SessionsPage
      batchError={batchMutation.isError ? formatApiErrorMessage(batchMutation.error, t) : null}
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
      openError={openMutation.isError ? formatApiErrorMessage(openMutation.error, t) : null}
      openResponse={openMutation.data ?? null}
      opening={openMutation.isPending}
      searchSessionOptions={async (payload) =>
        (await api.searchSessionOptions(profileId, payload)).items
      }
      sessions={sessionsQuery.data?.sessions ?? []}
      sessionsLoading={sessionsQuery.isLoading}
      suggestedPort={suggestedPortQuery.data?.port ?? null}
    />
  );
}
