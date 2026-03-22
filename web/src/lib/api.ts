import { buildTaskSearchParams } from "@/lib/tasks";
import type {
  AuthMeResponse,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
  CreateProfileRequest,
  CreateProfileResponse,
  ErrorResponse,
  ExtractIpRequest,
  ExtractIpResponse,
  HealthResponse,
  ListApiKeysResponse,
  ListProfilesResponse,
  ListSessionsResponse,
  LoadSubscriptionRequest,
  LoadSubscriptionResponse,
  OpenBatchRequest,
  OpenBatchResponse,
  OpenSessionRequest,
  OpenSessionResponse,
  RefreshRequest,
  RefreshResponse,
  TaskListQuery,
  TaskListResponse,
  TaskRunDetail,
} from "@/lib/types";

class ApiError extends Error {
  status: number;
  code: string;
  details?: unknown;

  constructor(status: number, payload: ErrorResponse) {
    super(payload.message);
    this.name = "ApiError";
    this.status = status;
    this.code = payload.code;
    this.details = payload.details;
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
    ...init,
  });

  if (!response.ok) {
    let payload: ErrorResponse = {
      code: `http_${response.status}`,
      message: response.statusText || "Request failed",
    };
    try {
      payload = (await response.json()) as ErrorResponse;
    } catch {
      // fallback to default payload
    }
    throw new ApiError(response.status, payload);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

const profilePath = (profileId: string, suffix: string) =>
  `/api/v1/profiles/${encodeURIComponent(profileId)}${suffix}`;

const withSearch = (path: string, query?: TaskListQuery) => {
  if (!query) {
    return path;
  }
  const params = buildTaskSearchParams(query);
  const suffix = params.toString();
  return suffix ? `${path}?${suffix}` : path;
};

export { ApiError };

export const api = {
  getHealth: () => request<HealthResponse>("/healthz"),
  getAuthMe: () => request<AuthMeResponse>("/api/v1/auth/me"),
  listProfiles: () => request<ListProfilesResponse>("/api/v1/profiles"),
  createProfile: (payload: CreateProfileRequest) =>
    request<CreateProfileResponse>("/api/v1/profiles", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  listTasks: (query?: TaskListQuery) =>
    request<TaskListResponse>(withSearch("/api/v1/tasks", query)),
  getTaskRunDetail: (runId: string) =>
    request<TaskRunDetail>(`/api/v1/tasks/${encodeURIComponent(runId)}`),
  getTaskEventsUrl: (query?: TaskListQuery) => withSearch("/api/v1/tasks/events", query),
  listSessions: (profileId: string) =>
    request<ListSessionsResponse>(profilePath(profileId, "/sessions")),
  loadSubscription: (profileId: string, payload: LoadSubscriptionRequest) =>
    request<LoadSubscriptionResponse>(profilePath(profileId, "/subscriptions/load"), {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  refreshProfile: (profileId: string, payload: RefreshRequest) =>
    request<RefreshResponse>(profilePath(profileId, "/refresh"), {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  extractIps: (profileId: string, payload: ExtractIpRequest) =>
    request<ExtractIpResponse>(profilePath(profileId, "/ips/extract"), {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  openSession: (profileId: string, payload: OpenSessionRequest) =>
    request<OpenSessionResponse>(profilePath(profileId, "/sessions/open"), {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  openBatch: (profileId: string, payload: OpenBatchRequest) =>
    request<OpenBatchResponse>(profilePath(profileId, "/sessions/open-batch"), {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  listApiKeys: (profileId: string) =>
    request<ListApiKeysResponse>(profilePath(profileId, "/api-keys")),
  createApiKey: (profileId: string, payload: CreateApiKeyRequest) =>
    request<CreateApiKeyResponse>(profilePath(profileId, "/api-keys"), {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  revokeApiKey: (profileId: string, keyId: string) =>
    request<void>(profilePath(profileId, `/api-keys/${encodeURIComponent(keyId)}`), {
      method: "DELETE",
    }),
  closeSession: (profileId: string, sessionId: string) =>
    request<void>(profilePath(profileId, `/sessions/${encodeURIComponent(sessionId)}`), {
      method: "DELETE",
    }),
};
