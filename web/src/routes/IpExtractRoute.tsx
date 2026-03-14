import { useMutation } from "@tanstack/react-query";
import { useEffect, useState } from "react";
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
  const [resultByProfile, setResultByProfile] = useState<
    Record<string, { request: ExtractIpRequest; response: ExtractIpResponse } | null>
  >({});

  const mutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.extractIps>[1]) =>
      api.extractIps(profileId, payload),
    onSuccess: (data, variables) => {
      setResultByProfile((current) => ({
        ...current,
        [profileId]: { request: variables, response: data },
      }));
      toast.success(`Extracted ${data.items.length} candidate IPs`);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  useEffect(() => {
    mutation.reset();
  }, [profileId]);

  return (
    <IpExtractPage
      error={mutation.isError ? getErrorMessage(mutation.error) : null}
      isPending={mutation.isPending}
      lastRequest={resultByProfile[profileId]?.request ?? null}
      onSubmit={async (payload) => {
        await mutation.mutateAsync(payload);
      }}
      response={resultByProfile[profileId]?.response ?? null}
    />
  );
}
