import { useMutation } from "@tanstack/react-query";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { ApiError, api } from "@/lib/api";
import { IpExtractPage } from "@/pages/IpExtractPage";
import type { RootOutletContext } from "@/routes/RootRoute";

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function IpExtractRoute() {
  const { profileId } = useOutletContext<RootOutletContext>();
  const mutation = useMutation({
    mutationFn: (payload: Parameters<typeof api.extractIps>[1]) =>
      api.extractIps(profileId, payload),
    onSuccess: (data) => {
      toast.success(`Extracted ${data.items.length} candidate IPs`);
    },
    onError: (error) => toast.error(getErrorMessage(error)),
  });

  return (
    <IpExtractPage
      error={mutation.isError ? getErrorMessage(mutation.error) : null}
      isPending={mutation.isPending}
      onSubmit={async (payload) => {
        await mutation.mutateAsync(payload);
      }}
      response={mutation.data ?? null}
    />
  );
}
