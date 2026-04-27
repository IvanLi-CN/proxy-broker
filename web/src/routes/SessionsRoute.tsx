import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { Navigate, useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import { resolveSessionDisplayAddress } from "@/lib/format";
import { isGlobalProfileId } from "@/lib/profile-selection";
import { SessionsPage } from "@/pages/SessionsPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function SessionsRoute() {
  const { t } = useI18n();
  const outlet = useOutletContext<RootOutletContext>();
  const { profileId } = outlet;
  const isGlobalConfig = outlet.isGlobalConfig ?? isGlobalProfileId(profileId);
  const activeProfileId = outlet.activeProfileId ?? (isGlobalConfig ? null : profileId);
  const previousProfileId = useRef(activeProfileId ?? "");
  const queryClient = useQueryClient();
  const sessionsQuery = useQuery({
    queryKey: ["sessions", activeProfileId],
    queryFn: () => api.listSessions(activeProfileId ?? ""),
    enabled: Boolean(activeProfileId),
    refetchInterval: 5_000,
  });
  const suggestedPortQuery = useQuery({
    queryKey: ["suggested-port", activeProfileId],
    queryFn: () => api.getSuggestedPort(activeProfileId ?? ""),
    enabled: Boolean(activeProfileId),
    refetchInterval: 5_000,
  });

  const openMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.openSessionByIp>[1]) =>
      api.openSessionByIp(activeProfileId ?? "", payload),
    onSuccess: async (data) => {
      toast.success(t("Opened {listen}", { listen: resolveSessionDisplayAddress(data) }));
      await queryClient.invalidateQueries({ queryKey: ["sessions", activeProfileId] });
      await queryClient.invalidateQueries({ queryKey: ["suggested-port", activeProfileId] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const batchMutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.openBatchByIp>[1]) =>
      api.openBatchByIp(activeProfileId ?? "", payload),
    onSuccess: async (data) => {
      toast.success(t("Opened {count} sessions in batch", { count: data.sessions.length }));
      await queryClient.invalidateQueries({ queryKey: ["sessions", activeProfileId] });
      await queryClient.invalidateQueries({ queryKey: ["suggested-port", activeProfileId] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const switchMutation = useMutation({
    mutationFn: ({
      sessionId,
      payload,
    }: {
      sessionId: string;
      payload: Parameters<typeof api.updateSessionNode>[2];
    }) => api.updateSessionNode(activeProfileId ?? "", sessionId, payload),
    onSuccess: async (data, variables) => {
      toast.success(
        t("Switched {sessionId} to {proxyName}", {
          sessionId: variables.sessionId,
          proxyName: data.proxy_name,
        }),
      );
      await queryClient.invalidateQueries({ queryKey: ["sessions", activeProfileId] });
    },
  });

  const closeMutation = useMutation({
    mutationFn: (sessionId: string) => api.closeSession(activeProfileId ?? "", sessionId),
    onSuccess: async (_, sessionId) => {
      toast.success(t("Closed {sessionId}", { sessionId }));
      await queryClient.invalidateQueries({ queryKey: ["sessions", activeProfileId] });
      await queryClient.invalidateQueries({ queryKey: ["suggested-port", activeProfileId] });
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const { reset: resetOpenMutation } = openMutation;
  const { reset: resetBatchMutation } = batchMutation;
  const { reset: resetSwitchMutation } = switchMutation;
  const { reset: resetCloseMutation } = closeMutation;

  useEffect(() => {
    if (!activeProfileId) {
      return;
    }
    if (previousProfileId.current === activeProfileId) {
      return;
    }
    previousProfileId.current = activeProfileId;
    resetOpenMutation();
    resetBatchMutation();
    resetSwitchMutation();
    resetCloseMutation();
  }, [
    activeProfileId,
    resetBatchMutation,
    resetCloseMutation,
    resetOpenMutation,
    resetSwitchMutation,
  ]);

  if (!activeProfileId) {
    return <Navigate replace to="/proxies" />;
  }

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
      onResetCreateState={() => {
        resetOpenMutation();
        resetBatchMutation();
      }}
      onResetSwitchState={() => {
        resetSwitchMutation();
      }}
      onUpdateSessionNode={async (sessionId, payload) => {
        await switchMutation.mutateAsync({ sessionId, payload });
      }}
      openError={openMutation.isError ? formatApiErrorMessage(openMutation.error, t) : null}
      openResponse={openMutation.data ?? null}
      opening={openMutation.isPending}
      searchSessionIpNodeOptions={async (payload) =>
        (await api.searchSessionIpNodeOptions(activeProfileId, payload)).groups
      }
      sessions={sessionsQuery.data?.sessions ?? []}
      sessionsLoading={sessionsQuery.isLoading}
      suggestedPort={suggestedPortQuery.data?.port ?? null}
      switchError={switchMutation.isError ? formatApiErrorMessage(switchMutation.error, t) : null}
      switchedSessionId={switchMutation.data?.session_id ?? null}
      switchingSessionId={
        switchMutation.isPending ? (switchMutation.variables?.sessionId ?? null) : null
      }
    />
  );
}
