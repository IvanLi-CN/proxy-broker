import { useEffect, useState } from "react";

import { DEFAULT_PROFILE_ID } from "@/lib/profile-selection";

const STORAGE_KEY = "proxy-broker.profile-id";

export function useProfilePreference() {
  const [profileId, setProfileId] = useState(() => {
    if (typeof window === "undefined") {
      return DEFAULT_PROFILE_ID;
    }
    return window.localStorage.getItem(STORAGE_KEY) || DEFAULT_PROFILE_ID;
  });

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, profileId);
  }, [profileId]);

  return [profileId, setProfileId] as const;
}
