import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Outlet } from "react-router-dom";
import { toast } from "sonner";

import { AppShell } from "@/components/AppShell";
import { useProfilePreference } from "@/hooks/use-profile-preference";
import { ApiError, api } from "@/lib/api";

export interface RootOutletContext {
  profileId: string;
}

const getErrorMessage = (error: unknown) =>
  error instanceof ApiError ? `${error.code}: ${error.message}` : "Unexpected request error";

export function RootRoute() {
  const [profileId, setProfileId] = useProfilePreference();
  const queryClient = useQueryClient();
  const healthQuery = useQuery({
    queryKey: ["health"],
    queryFn: api.getHealth,
    refetchInterval: 10_000,
  });
  const profilesQuery = useQuery({
    queryKey: ["profiles"],
    queryFn: api.listProfiles,
  });
  const createProfileMutation = useMutation({
    mutationFn: (nextProfileId: string) => api.createProfile({ profile_id: nextProfileId }),
  });
  const profiles = Array.from(
    new Set([...(profilesQuery.data?.profiles ?? []), profileId].filter(Boolean)),
  ).sort((left, right) => left.localeCompare(right));

  const handleCreateProfile = async (nextProfileId: string) => {
    try {
      const created = await createProfileMutation.mutateAsync(nextProfileId);
      setProfileId(created.profile_id);
      toast.success(`Created profile ${created.profile_id}`);
      await queryClient.invalidateQueries({ queryKey: ["profiles"] });
      return created.profile_id;
    } catch (error) {
      if (error instanceof ApiError && error.code === "profile_exists") {
        const existingProfileId = nextProfileId.trim();
        setProfileId(existingProfileId);
        toast.info(`Profile ${existingProfileId} already exists. Switched to it instead.`);
        await queryClient.invalidateQueries({ queryKey: ["profiles"] });
        return existingProfileId;
      }
      toast.error(getErrorMessage(error));
      throw error;
    }
  };

  return (
    <AppShell
      healthStatus={healthQuery.data?.status ?? "checking"}
      onCreateProfile={handleCreateProfile}
      onProfileIdChange={setProfileId}
      onRetryProfiles={() => {
        void profilesQuery.refetch();
      }}
      profiles={profiles}
      profilesCreating={createProfileMutation.isPending}
      profilesError={profilesQuery.isError ? getErrorMessage(profilesQuery.error) : null}
      profilesLoading={profilesQuery.isLoading}
      profileId={profileId}
    >
      <Outlet context={{ profileId } satisfies RootOutletContext} />
    </AppShell>
  );
}
