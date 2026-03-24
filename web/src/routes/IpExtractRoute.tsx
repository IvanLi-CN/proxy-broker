import { useMutation } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import type { ExtractIpRequest, ExtractIpResponse } from "@/lib/types";
import { IpExtractPage } from "@/pages/IpExtractPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function IpExtractRoute() {
  const { t } = useI18n();
  const { profileId } = useOutletContext<RootOutletContext>();
  const previousProfileId = useRef(profileId);
  const [resultByProfile, setResultByProfile] = useState<
    Record<string, { request: ExtractIpRequest; response: ExtractIpResponse } | null>
  >({});

  const mutation = useMutation({
    mutationFn: ({
      profileId: requestedProfileId,
      payload,
    }: {
      profileId: string;
      payload: Parameters<typeof api.extractIps>[1];
    }) => api.extractIps(requestedProfileId, payload),
    onSuccess: (data, { profileId: requestedProfileId, payload }) => {
      setResultByProfile((current) => ({
        ...current,
        [requestedProfileId]: { request: payload, response: data },
      }));
      toast.success(t("Extracted {count} candidate IPs", { count: data.items.length }));
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const { reset: resetMutation } = mutation;

  useEffect(() => {
    if (previousProfileId.current === profileId) {
      return;
    }
    previousProfileId.current = profileId;
    resetMutation();
  }, [profileId, resetMutation]);

  return (
    <IpExtractPage
      error={mutation.isError ? formatApiErrorMessage(mutation.error, t) : null}
      isPending={mutation.isPending}
      lastRequest={resultByProfile[profileId]?.request ?? null}
      onSubmit={async (payload) => {
        await mutation.mutateAsync({ profileId, payload });
      }}
      response={resultByProfile[profileId]?.response ?? null}
    />
  );
}
