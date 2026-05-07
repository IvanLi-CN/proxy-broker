import { useEffect, useState } from "react";

import { DEFAULT_PROJECT_ID } from "@/lib/project-selection";

const STORAGE_KEY = "proxy-broker.project-id";

export function useProjectPreference() {
  const [projectId, setProjectId] = useState(() => {
    if (typeof window === "undefined") {
      return DEFAULT_PROJECT_ID;
    }
    return window.localStorage.getItem(STORAGE_KEY) || DEFAULT_PROJECT_ID;
  });

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, projectId);
  }, [projectId]);

  return [projectId, setProjectId] as const;
}
