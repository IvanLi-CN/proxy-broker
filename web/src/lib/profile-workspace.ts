import { useCallback, useEffect, useMemo, useState } from "react";

import {
  DEFAULT_IP_FILTERS_FORM_VALUES,
  type IpFiltersFormValues,
} from "@/features/ips/components/IpFiltersForm";
import {
  DEFAULT_REFRESH_FORM_VALUES,
  type RefreshFormValues,
} from "@/features/overview/components/RefreshCard";
import {
  DEFAULT_SUBSCRIPTION_FORM_VALUES,
  type SubscriptionFormValues,
} from "@/features/overview/components/SubscriptionFormCard";
import {
  emptyBatchRequestRow,
  type OpenBatchFormValues,
} from "@/features/sessions/components/OpenBatchForm";
import {
  DEFAULT_OPEN_SESSION_FORM_VALUES,
  type OpenSessionFormValues,
} from "@/features/sessions/components/OpenSessionForm";
import type {
  ExtractIpRequest,
  ExtractIpResponse,
  LoadSubscriptionResponse,
  OpenBatchResponse,
  OpenSessionResponse,
  RefreshResponse,
} from "@/lib/types";

const STORAGE_KEY = "proxy-broker.profile-workspace";
const STORAGE_VERSION = 1;
export const DEFAULT_PROFILE_ID = "default";
const MAX_RECENT_PROFILES = 10;

export interface OverviewWorkspaceState {
  subscriptionForm: SubscriptionFormValues;
  refreshForm: RefreshFormValues;
  loadResponse: LoadSubscriptionResponse | null;
  refreshResponse: RefreshResponse | null;
}

export interface IpExtractWorkspaceState {
  filtersForm: IpFiltersFormValues;
  lastRequest: ExtractIpRequest | null;
  response: ExtractIpResponse | null;
}

export interface SessionsWorkspaceState {
  openForm: OpenSessionFormValues;
  batchForm: OpenBatchFormValues;
  openResponse: OpenSessionResponse | null;
  batchResponse: OpenBatchResponse | null;
}

export interface ProfileWorkspaceState {
  overview: OverviewWorkspaceState;
  ipExtract: IpExtractWorkspaceState;
  sessions: SessionsWorkspaceState;
}

interface WorkspaceStorageState {
  version: number;
  activeProfileId: string;
  recentProfileIds: string[];
  profiles: Record<string, ProfileWorkspaceState>;
}

const defaultBatchForm = (): OpenBatchFormValues => ({
  requests: [
    emptyBatchRequestRow(),
    { ...emptyBatchRequestRow(), desiredPort: "10081", cities: "Osaka" },
  ],
});

const defaultOverviewState = (): OverviewWorkspaceState => ({
  subscriptionForm: { ...DEFAULT_SUBSCRIPTION_FORM_VALUES },
  refreshForm: { ...DEFAULT_REFRESH_FORM_VALUES },
  loadResponse: null,
  refreshResponse: null,
});

const defaultIpExtractState = (): IpExtractWorkspaceState => ({
  filtersForm: { ...DEFAULT_IP_FILTERS_FORM_VALUES },
  lastRequest: null,
  response: null,
});

const defaultSessionsState = (): SessionsWorkspaceState => ({
  openForm: { ...DEFAULT_OPEN_SESSION_FORM_VALUES },
  batchForm: defaultBatchForm(),
  openResponse: null,
  batchResponse: null,
});

const defaultProfileWorkspaceState = (): ProfileWorkspaceState => ({
  overview: defaultOverviewState(),
  ipExtract: defaultIpExtractState(),
  sessions: defaultSessionsState(),
});

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function normalizeProfileId(rawValue: string): string {
  return rawValue
    .trim()
    .toLowerCase()
    .replace(/[\s_]+/g, "-")
    .replace(/[^a-z0-9-]/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function mergeRecentProfileIds(current: string[], nextProfileId: string): string[] {
  return [nextProfileId, ...current.filter((item) => item !== nextProfileId)].slice(
    0,
    MAX_RECENT_PROFILES,
  );
}

function sanitizeProfileWorkspace(
  value: Partial<ProfileWorkspaceState> | undefined,
): ProfileWorkspaceState {
  return {
    overview: {
      ...defaultOverviewState(),
      ...(value?.overview ?? {}),
      subscriptionForm: {
        ...DEFAULT_SUBSCRIPTION_FORM_VALUES,
        ...(value?.overview?.subscriptionForm ?? {}),
      },
      refreshForm: {
        ...DEFAULT_REFRESH_FORM_VALUES,
        ...(value?.overview?.refreshForm ?? {}),
      },
    },
    ipExtract: {
      ...defaultIpExtractState(),
      ...(value?.ipExtract ?? {}),
      filtersForm: {
        ...DEFAULT_IP_FILTERS_FORM_VALUES,
        ...(value?.ipExtract?.filtersForm ?? {}),
      },
    },
    sessions: {
      ...defaultSessionsState(),
      ...(value?.sessions ?? {}),
      openForm: {
        ...DEFAULT_OPEN_SESSION_FORM_VALUES,
        ...(value?.sessions?.openForm ?? {}),
      },
      batchForm: value?.sessions?.batchForm ? clone(value.sessions.batchForm) : defaultBatchForm(),
    },
  };
}

function buildDefaultStorageState(): WorkspaceStorageState {
  return {
    version: STORAGE_VERSION,
    activeProfileId: DEFAULT_PROFILE_ID,
    recentProfileIds: [DEFAULT_PROFILE_ID],
    profiles: {
      [DEFAULT_PROFILE_ID]: defaultProfileWorkspaceState(),
    },
  };
}

function sanitizeStorageState(
  rawState: Partial<WorkspaceStorageState> | null | undefined,
): WorkspaceStorageState {
  const baseState = buildDefaultStorageState();
  const profileEntries = Object.entries(rawState?.profiles ?? {}).reduce<
    Record<string, ProfileWorkspaceState>
  >((accumulator, [rawProfileId, workspace]) => {
    const profileId = normalizeProfileId(rawProfileId);
    if (!profileId) {
      return accumulator;
    }
    accumulator[profileId] = sanitizeProfileWorkspace(workspace);
    return accumulator;
  }, {});

  const activeProfileId = normalizeProfileId(rawState?.activeProfileId ?? "") || DEFAULT_PROFILE_ID;
  const recentProfileIds = Array.from(
    new Set(
      (rawState?.recentProfileIds ?? [])
        .map((item) => normalizeProfileId(item))
        .filter((item) => item.length > 0),
    ),
  );

  const profiles = {
    ...baseState.profiles,
    ...profileEntries,
  };

  if (!profiles[activeProfileId]) {
    profiles[activeProfileId] = defaultProfileWorkspaceState();
  }

  return {
    version: STORAGE_VERSION,
    activeProfileId,
    recentProfileIds: mergeRecentProfileIds(recentProfileIds, activeProfileId),
    profiles,
  };
}

function readStorageState(): WorkspaceStorageState {
  if (typeof window === "undefined") {
    return buildDefaultStorageState();
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return buildDefaultStorageState();
    }
    return sanitizeStorageState(JSON.parse(raw) as Partial<WorkspaceStorageState>);
  } catch {
    return buildDefaultStorageState();
  }
}

function writeStorageState(nextState: WorkspaceStorageState) {
  if (typeof window === "undefined") {
    return;
  }
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(nextState));
}

function updateStorageState(
  current: WorkspaceStorageState,
  updater: (draft: WorkspaceStorageState) => WorkspaceStorageState,
): WorkspaceStorageState {
  const next = sanitizeStorageState(updater(current));
  writeStorageState(next);
  return next;
}

export function useProfileWorkspacePreferences() {
  const [storage, setStorage] = useState<WorkspaceStorageState>(() => readStorageState());

  useEffect(() => {
    writeStorageState(storage);
  }, [storage]);

  const setActiveProfileId = useCallback((rawProfileId: string) => {
    const profileId = normalizeProfileId(rawProfileId);
    if (!profileId) {
      return false;
    }
    setStorage((current) =>
      updateStorageState(current, (draft) => ({
        ...draft,
        activeProfileId: profileId,
        recentProfileIds: mergeRecentProfileIds(draft.recentProfileIds, profileId),
        profiles: {
          ...draft.profiles,
          [profileId]: draft.profiles[profileId] ?? defaultProfileWorkspaceState(),
        },
      })),
    );
    return true;
  }, []);

  const updateProfileWorkspace = useCallback(
    <K extends keyof ProfileWorkspaceState>(
      profileId: string,
      key: K,
      updater: (value: ProfileWorkspaceState[K]) => ProfileWorkspaceState[K],
    ) => {
      const normalizedProfileId = normalizeProfileId(profileId) || DEFAULT_PROFILE_ID;
      setStorage((current) =>
        updateStorageState(current, (draft) => {
          const currentWorkspace =
            draft.profiles[normalizedProfileId] ?? defaultProfileWorkspaceState();
          return {
            ...draft,
            profiles: {
              ...draft.profiles,
              [normalizedProfileId]: {
                ...currentWorkspace,
                [key]: updater(clone(currentWorkspace[key])),
              },
            },
          };
        }),
      );
    },
    [],
  );

  const ensureProfileWorkspace = useCallback(
    (profileId: string) =>
      storage.profiles[normalizeProfileId(profileId) || DEFAULT_PROFILE_ID] ??
      defaultProfileWorkspaceState(),
    [storage.profiles],
  );

  const workspace = useMemo(
    () => ensureProfileWorkspace(storage.activeProfileId),
    [ensureProfileWorkspace, storage.activeProfileId],
  );

  return {
    activeProfileId: storage.activeProfileId,
    recentProfileIds: storage.recentProfileIds,
    workspace,
    ensureProfileWorkspace,
    setActiveProfileId,
    updateProfileWorkspace,
  };
}
