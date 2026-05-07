import { useMutation } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { Navigate, useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import { isGlobalProjectId } from "@/lib/project-selection";
import type { ExtractIpRequest, ExtractIpResponse } from "@/lib/types";
import { IpExtractPage } from "@/pages/IpExtractPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function IpExtractRoute() {
  const { t } = useI18n();
  const outlet = useOutletContext<RootOutletContext>();
  const { projectId } = outlet;
  const isGlobalProject = outlet.isGlobalProject ?? isGlobalProjectId(projectId);
  const activeProjectId = outlet.activeProjectId ?? (isGlobalProject ? null : projectId);
  const previousProjectId = useRef(activeProjectId ?? "");
  const [resultByProject, setResultByProject] = useState<
    Record<string, { request: ExtractIpRequest; response: ExtractIpResponse } | null>
  >({});

  const mutation = useMutation({
    mutationFn: ({
      projectId: requestedProjectId,
      payload,
    }: {
      projectId: string;
      payload: Parameters<typeof api.extractIps>[1];
    }) => api.extractIps(requestedProjectId, payload),
    onSuccess: (data, { projectId: requestedProjectId, payload }) => {
      setResultByProject((current) => ({
        ...current,
        [requestedProjectId]: { request: payload, response: data },
      }));
      toast.success(t("Extracted {count} candidate IPs", { count: data.items.length }));
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const { reset: resetMutation } = mutation;

  useEffect(() => {
    if (!activeProjectId) {
      return;
    }
    if (previousProjectId.current === activeProjectId) {
      return;
    }
    previousProjectId.current = activeProjectId;
    resetMutation();
  }, [activeProjectId, resetMutation]);

  if (!activeProjectId) {
    return <Navigate replace to="/proxies" />;
  }

  return (
    <IpExtractPage
      error={mutation.isError ? formatApiErrorMessage(mutation.error, t) : null}
      isPending={mutation.isPending}
      lastRequest={resultByProject[activeProjectId]?.request ?? null}
      onSubmit={async (payload) => {
        await mutation.mutateAsync({ projectId: activeProjectId, payload });
      }}
      response={resultByProject[activeProjectId]?.response ?? null}
    />
  );
}
