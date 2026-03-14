import { useMutation } from "@tanstack/react-query";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { ApiError, api } from "@/lib/api";
import { IpExtractPage } from "@/pages/IpExtractPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function IpExtractRoute() {
  const {
    profileId,
    profileSummary,
    profileSummaryLoading,
    ipExtractWorkspace,
    updateIpExtractWorkspace,
    writeIpExtractWorkspace,
  } = useOutletContext<RootOutletContext>();

  const mutation = useMutation({
    mutationFn: ({
      profileId: requestedProfileId,
      payload,
    }: {
      profileId: string;
      payload: Parameters<typeof api.extractIps>[1];
    }) => api.extractIps(requestedProfileId, payload),
    onSuccess: (data, { profileId: requestedProfileId, payload }) => {
      writeIpExtractWorkspace(requestedProfileId, (current) => ({
        ...current,
        lastRequest: payload,
        response: data,
      }));
      toast.success(`Extracted ${data.items.length} candidate IPs`);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  return (
    <IpExtractPage
      error={mutation.isError ? getErrorMessage(mutation.error) : null}
      filtersFormValues={ipExtractWorkspace.filtersForm}
      initialized={profileSummary?.initialized ?? false}
      initializationLoading={profileSummaryLoading}
      isPending={mutation.isPending}
      lastRequest={ipExtractWorkspace.lastRequest}
      onFormValuesChange={(values) =>
        updateIpExtractWorkspace((current) => ({ ...current, filtersForm: values }))
      }
      onSubmit={async (payload) => {
        await mutation.mutateAsync({ profileId, payload });
      }}
      profileId={profileId}
      response={ipExtractWorkspace.response}
    />
  );
}
