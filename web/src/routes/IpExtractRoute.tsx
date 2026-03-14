import { useMutation } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { ApiError, api } from "@/lib/api";
import type { ExtractIpRequest, ExtractIpResponse } from "@/lib/types";
import { IpExtractPage } from "@/pages/IpExtractPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function IpExtractRoute() {
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
      toast.success(`Extracted ${data.items.length} candidate IPs`);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
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
      error={mutation.isError ? getErrorMessage(mutation.error) : null}
      isPending={mutation.isPending}
      lastRequest={resultByProfile[profileId]?.request ?? null}
      onSubmit={async (payload) => {
        await mutation.mutateAsync({ profileId, payload });
      }}
      response={resultByProfile[profileId]?.response ?? null}
    />
  );
}
