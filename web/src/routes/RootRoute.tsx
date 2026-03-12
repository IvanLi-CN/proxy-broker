import { useQuery } from "@tanstack/react-query";
import { Outlet } from "react-router-dom";

import { AppShell } from "@/components/AppShell";
import { useProfilePreference } from "@/hooks/use-profile-preference";
import { api } from "@/lib/api";

export interface RootOutletContext {
  profileId: string;
}

export function RootRoute() {
  const [profileId, setProfileId] = useProfilePreference();
  const healthQuery = useQuery({
    queryKey: ["health"],
    queryFn: api.getHealth,
    refetchInterval: 10_000,
  });

  return (
    <AppShell
      healthStatus={healthQuery.data?.status ?? "checking"}
      onProfileIdChange={setProfileId}
      profileId={profileId}
    >
      <Outlet context={{ profileId } satisfies RootOutletContext} />
    </AppShell>
  );
}
