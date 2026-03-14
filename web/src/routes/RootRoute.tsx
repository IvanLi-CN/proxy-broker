import { useQuery } from "@tanstack/react-query";
import { Outlet } from "react-router-dom";

import { AppShell } from "@/components/AppShell";
import { api } from "@/lib/api";
import {
  type IpExtractWorkspaceState,
  type OverviewWorkspaceState,
  type SessionsWorkspaceState,
  useProfileWorkspacePreferences,
} from "@/lib/profile-workspace";
import type { ProfileSummaryResponse } from "@/lib/types";

export interface RootOutletContext {
  profileId: string;
  profileSummary: ProfileSummaryResponse | null;
  profileSummaryLoading: boolean;
  overviewWorkspace: OverviewWorkspaceState;
  ipExtractWorkspace: IpExtractWorkspaceState;
  sessionsWorkspace: SessionsWorkspaceState;
  updateOverviewWorkspace: (
    updater: (value: OverviewWorkspaceState) => OverviewWorkspaceState,
  ) => void;
  updateIpExtractWorkspace: (
    updater: (value: IpExtractWorkspaceState) => IpExtractWorkspaceState,
  ) => void;
  updateSessionsWorkspace: (
    updater: (value: SessionsWorkspaceState) => SessionsWorkspaceState,
  ) => void;
}

function mergeKnownProfiles(
  activeProfileId: string,
  recentProfileIds: string[],
  remoteProfiles: string[],
) {
  const nextProfiles = [
    activeProfileId,
    ...recentProfileIds.filter((item) => item !== activeProfileId),
  ];
  const remainingProfiles = remoteProfiles
    .filter((item) => item !== activeProfileId && !recentProfileIds.includes(item))
    .sort((left, right) => left.localeCompare(right));

  return Array.from(new Set([...nextProfiles, ...remainingProfiles]));
}

export function RootRoute() {
  const {
    activeProfileId,
    recentProfileIds,
    workspace,
    setActiveProfileId,
    updateProfileWorkspace,
  } = useProfileWorkspacePreferences();

  const healthQuery = useQuery({
    queryKey: ["health"],
    queryFn: api.getHealth,
    refetchInterval: 10_000,
  });
  const profilesQuery = useQuery({
    queryKey: ["profiles"],
    queryFn: async () => (await api.listProfiles()).profiles,
  });
  const summaryQuery = useQuery({
    queryKey: ["profile-summary", activeProfileId],
    queryFn: () => api.getProfileSummary(activeProfileId),
  });

  const knownProfiles = mergeKnownProfiles(
    activeProfileId,
    recentProfileIds,
    profilesQuery.data ?? [],
  );

  return (
    <AppShell
      healthStatus={healthQuery.data?.status ?? "checking"}
      knownProfiles={knownProfiles}
      onProfileIdChange={setActiveProfileId}
      profileId={activeProfileId}
      profileLoading={summaryQuery.isFetching}
      profilesLoading={profilesQuery.isLoading}
      recentProfileIds={recentProfileIds}
    >
      <Outlet
        context={
          {
            profileId: activeProfileId,
            profileSummary: summaryQuery.data ?? null,
            profileSummaryLoading: summaryQuery.isFetching,
            overviewWorkspace: workspace.overview,
            ipExtractWorkspace: workspace.ipExtract,
            sessionsWorkspace: workspace.sessions,
            updateOverviewWorkspace: (updater) =>
              updateProfileWorkspace(activeProfileId, "overview", updater),
            updateIpExtractWorkspace: (updater) =>
              updateProfileWorkspace(activeProfileId, "ipExtract", updater),
            updateSessionsWorkspace: (updater) =>
              updateProfileWorkspace(activeProfileId, "sessions", updater),
          } satisfies RootOutletContext
        }
      />
    </AppShell>
  );
}
