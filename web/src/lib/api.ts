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
  ListProxyImportResponse,
  ListProxyInventoryResponse,
  ListSessionsResponse,
  LoadSubscriptionRequest,
  LoadSubscriptionResponse,
  OpenBatchByIpRequest,
  OpenBatchByNodeRequest,
  OpenBatchRequest,
  OpenBatchResponse,
  OpenSessionByIpRequest,
  OpenSessionByNodeRequest,
  OpenSessionRequest,
  OpenSessionResponse,
  ProfileProxySettings,
  ProxyCatalogQuery,
  ProxyCatalogResponse,
  ProxyImportListQuery,
  ProxyInventoryListQuery,
  ProxyOperationAcceptedResponse,
  ProxyOperationRequest,
  RefreshRequest,
  RefreshResponse,
  SearchSessionIpNodeOptionsRequest,
  SearchSessionIpNodeOptionsResponse,
  SearchSessionNodeOptionsRequest,
  SearchSessionNodeOptionsResponse,
  SearchSessionOptionsRequest,
  SearchSessionOptionsResponse,
  SuggestedPortResponse,
  TaskListQuery,
  TaskListResponse,
  TaskRunDetail,
  UpdateProfileProxySettingsRequest,
  UpdateProxyAllocationRequest,
  UpdateProxyImportAllocationRequest,
  UpdateSessionNodeRequest,
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

const sessionDisplayHostHeader = "X-Proxy-Broker-Display-Host";

function getWindowHostname() {
  if (typeof window === "undefined") {
    return null;
  }
  const hostname = window.location.hostname?.trim();
  return hostname ? hostname : null;
}

function withSessionDisplayHostHeader(init?: RequestInit): RequestInit | undefined {
  const hostname = getWindowHostname();
  if (!hostname) {
    return init;
  }
  return {
    ...(init ?? {}),
    headers: {
      [sessionDisplayHostHeader]: hostname,
      ...(init?.headers ?? {}),
    },
  };
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
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

const withProxyInventorySearch = (path: string, query?: ProxyInventoryListQuery) => {
  if (!query) {
    return path;
  }

  const params = new URLSearchParams();
  if (query.scope) {
    params.set("scope", query.scope);
  }
  if (query.profile_id) {
    params.set("profile_id", query.profile_id);
  }
  const suffix = params.toString();
  return suffix ? `${path}?${suffix}` : path;
};

const withProxyImportSearch = (path: string, query?: ProxyImportListQuery) => {
  if (!query) {
    return path;
  }

  const params = new URLSearchParams();
  if (query.scope) {
    params.set("scope", query.scope);
  }
  if (query.profile_id) {
    params.set("profile_id", query.profile_id);
  }
  const suffix = params.toString();
  return suffix ? `${path}?${suffix}` : path;
};

const withProxyCatalogSearch = (path: string, query?: ProxyCatalogQuery) => {
  if (!query) {
    return path;
  }

  const params = new URLSearchParams();
  if (query.view) {
    params.set("view", query.view);
  }
  if (query.profile_id) {
    params.set("profile_id", query.profile_id);
  }
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
    request<ListSessionsResponse>(
      profilePath(profileId, "/sessions"),
      withSessionDisplayHostHeader(),
    ),
  loadSubscription: (profileId: string, payload: LoadSubscriptionRequest) =>
    request<LoadSubscriptionResponse>(profilePath(profileId, "/subscriptions/load"), {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  loadGlobalSubscription: (payload: LoadSubscriptionRequest) =>
    request<LoadSubscriptionResponse>("/api/v1/proxies/global/subscriptions/load", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  listProxyImports: (query?: ProxyImportListQuery) =>
    request<ListProxyImportResponse>(withProxyImportSearch("/api/v1/proxy-imports", query)),
  listProxyCatalog: (query?: ProxyCatalogQuery) =>
    request<ProxyCatalogResponse>(withProxyCatalogSearch("/api/v1/proxy-catalog", query)),
  refreshProxyCatalogMetadata: (payload: ProxyOperationRequest) =>
    request<ProxyOperationAcceptedResponse>("/api/v1/proxy-ops/refresh", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  probeProxyCatalogLatency: (payload: ProxyOperationRequest) =>
    request<ProxyOperationAcceptedResponse>("/api/v1/proxy-ops/probe", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  updateProxyImportAllocation: (importId: string, payload: UpdateProxyImportAllocationRequest) =>
    request<void>(`/api/v1/proxy-imports/${encodeURIComponent(importId)}/allocation`, {
      method: "PATCH",
      body: JSON.stringify(payload),
    }),
  deleteProxyImport: (importId: string) =>
    request<void>(`/api/v1/proxy-imports/${encodeURIComponent(importId)}`, {
      method: "DELETE",
    }),
  listProxyInventory: (query?: ProxyInventoryListQuery) =>
    request<ListProxyInventoryResponse>(withProxyInventorySearch("/api/v1/proxies", query)),
  updateProxyAllocation: (nodeId: string, payload: UpdateProxyAllocationRequest) =>
    request<void>(`/api/v1/proxies/${encodeURIComponent(nodeId)}/allocation`, {
      method: "PATCH",
      body: JSON.stringify(payload),
    }),
  deleteProxyInventoryNode: (nodeId: string) =>
    request<void>(`/api/v1/proxies/${encodeURIComponent(nodeId)}`, {
      method: "DELETE",
    }),
  getProfileProxySettings: (profileId: string) =>
    request<ProfileProxySettings>(profilePath(profileId, "/proxy-settings")),
  updateProfileProxySettings: (profileId: string, payload: UpdateProfileProxySettingsRequest) =>
    request<ProfileProxySettings>(profilePath(profileId, "/proxy-settings"), {
      method: "PATCH",
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
    request<OpenSessionResponse>(
      profilePath(profileId, "/sessions/open"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  openBatch: (profileId: string, payload: OpenBatchRequest) =>
    request<OpenBatchResponse>(
      profilePath(profileId, "/sessions/open-batch"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  openSessionByNode: (profileId: string, payload: OpenSessionByNodeRequest) =>
    request<OpenSessionResponse>(
      profilePath(profileId, "/sessions/open-by-node"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  openBatchByNode: (profileId: string, payload: OpenBatchByNodeRequest) =>
    request<OpenBatchResponse>(
      profilePath(profileId, "/sessions/open-batch-by-node"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  openSessionByIp: (profileId: string, payload: OpenSessionByIpRequest) =>
    request<OpenSessionResponse>(
      profilePath(profileId, "/sessions/open-by-ip"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  openBatchByIp: (profileId: string, payload: OpenBatchByIpRequest) =>
    request<OpenBatchResponse>(
      profilePath(profileId, "/sessions/open-batch-by-ip"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  updateSessionNode: (profileId: string, sessionId: string, payload: UpdateSessionNodeRequest) =>
    request<OpenSessionResponse>(
      profilePath(profileId, `/sessions/${encodeURIComponent(sessionId)}/node`),
      withSessionDisplayHostHeader({
        method: "PATCH",
        body: JSON.stringify(payload),
      }),
    ),
  getSuggestedPort: (profileId: string) =>
    request<SuggestedPortResponse>(profilePath(profileId, "/sessions/suggested-port")),
  searchSessionOptions: (profileId: string, payload: SearchSessionOptionsRequest) =>
    request<SearchSessionOptionsResponse>(profilePath(profileId, "/ips/options/search"), {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  searchSessionNodeOptions: (
    profileId: string,
    sessionId: string,
    payload: SearchSessionNodeOptionsRequest,
  ) =>
    request<SearchSessionNodeOptionsResponse>(
      profilePath(profileId, `/sessions/${encodeURIComponent(sessionId)}/node-options/search`),
      {
        method: "POST",
        body: JSON.stringify(payload),
      },
    ),
  searchSessionIpNodeOptions: (profileId: string, payload: SearchSessionIpNodeOptionsRequest) =>
    request<SearchSessionIpNodeOptionsResponse>(
      profilePath(profileId, "/sessions/ip-node-options/search"),
      {
        method: "POST",
        body: JSON.stringify(payload),
      },
    ),
  listApiKeys: () => request<ListApiKeysResponse>("/api/v1/api-keys"),
  createApiKey: (payload: CreateApiKeyRequest) =>
    request<CreateApiKeyResponse>("/api/v1/api-keys", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  revokeApiKey: (keyId: string) =>
    request<void>(`/api/v1/api-keys/${encodeURIComponent(keyId)}`, {
      method: "DELETE",
    }),
  closeSession: (profileId: string, sessionId: string) =>
    request<void>(profilePath(profileId, `/sessions/${encodeURIComponent(sessionId)}`), {
      method: "DELETE",
    }),
};
