import { useEffect, useState } from "react";

const STORAGE_KEY = "proxy-broker.profile-id";
const DEFAULT_PROFILE = "default";

export function useProfilePreference() {
  const [profileId, setProfileId] = useState(() => {
    if (typeof window === "undefined") {
      return DEFAULT_PROFILE;
    }
    return window.localStorage.getItem(STORAGE_KEY) || DEFAULT_PROFILE;
  });

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, profileId);
  }, [profileId]);

  return [profileId, setProfileId] as const;
}
