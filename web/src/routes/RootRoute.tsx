import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Outlet } from "react-router-dom";
import { toast } from "sonner";

import { AppShell } from "@/components/AppShell";
import { useProfilePreference } from "@/hooks/use-profile-preference";
import { useI18n } from "@/i18n";
import { ApiError, api } from "@/lib/api";
import { resolveCurrentUserState } from "@/lib/current-user";
import { formatApiErrorMessage } from "@/lib/error-messages";
import type { AuthMeResponse, CurrentUserState } from "@/lib/types";

export interface RootOutletContext {
  profileId: string;
  authMe: AuthMeResponse | null;
  currentUser: CurrentUserState;
}

export function RootRoute() {
  const { t } = useI18n();
  const [profileId, setProfileId] = useProfilePreference();
  const queryClient = useQueryClient();
  const healthQuery = useQuery({
    queryKey: ["health"],
    queryFn: api.getHealth,
    refetchInterval: 10_000,
  });
  const authMeQuery = useQuery({
    queryKey: ["auth-me"],
    queryFn: api.getAuthMe,
    refetchInterval: 30_000,
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
  const currentUser = resolveCurrentUserState({
    identity: authMeQuery.data ?? null,
    isLoading: authMeQuery.isLoading && !authMeQuery.data,
    error: authMeQuery.error ?? null,
  });

  const handleCreateProfile = async (nextProfileId: string) => {
    try {
      const created = await createProfileMutation.mutateAsync(nextProfileId);
      setProfileId(created.profile_id);
      toast.success(t("Created profile {profileId}", { profileId: created.profile_id }));
      await queryClient.invalidateQueries({ queryKey: ["profiles"] });
      return created.profile_id;
    } catch (error) {
      if (error instanceof ApiError && error.code === "profile_exists") {
        const existingProfileId = nextProfileId.trim();
        toast.info(
          t("Profile {profileId} already exists. Refreshing catalog.", {
            profileId: existingProfileId,
          }),
        );
        await queryClient.invalidateQueries({ queryKey: ["profiles"] });
      }
      toast.error(formatApiErrorMessage(error, t));
      throw error;
    }
  };

  return (
    <AppShell
      currentUser={currentUser}
      healthStatus={healthQuery.data?.status ?? "checking"}
      onCreateProfile={handleCreateProfile}
      onProfileIdChange={setProfileId}
      onRetryProfiles={() => {
        void profilesQuery.refetch();
      }}
      profiles={profiles}
      profilesCreating={createProfileMutation.isPending}
      profilesError={
        profilesQuery.isError && !profilesQuery.data
          ? formatApiErrorMessage(profilesQuery.error, t)
          : null
      }
      profilesLoading={profilesQuery.isLoading}
      profileId={profileId}
    >
      <Outlet
        context={
          { profileId, authMe: authMeQuery.data ?? null, currentUser } satisfies RootOutletContext
        }
      />
    </AppShell>
  );
}
