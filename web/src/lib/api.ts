import { buildTaskSearchParams } from "@/lib/tasks";
import type {
  AuthMeResponse,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
  CreateProjectRequest,
  CreateProjectResponse,
  ErrorResponse,
  ExtractIpRequest,
  ExtractIpResponse,
  HealthResponse,
  ListApiKeysResponse,
  ListProjectsResponse,
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
  ProjectProxySettings,
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
  SyncProxyImportsRequest,
  SyncProxyImportsResponse,
  SystemSettings,
  TaskListQuery,
  TaskListResponse,
  TaskRunDetail,
  UpdateProjectProxySettingsRequest,
  UpdateProxyAllocationRequest,
  UpdateProxyImportAllocationRequest,
  UpdateSessionNodeRequest,
  UpdateSystemSettingsRequest,
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

const projectPath = (projectId: string, suffix: string) =>
  `/api/v1/projects/${encodeURIComponent(projectId)}${suffix}`;

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
  if (query.project_id) {
    params.set("project_id", query.project_id);
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
  if (query.project_id) {
    params.set("project_id", query.project_id);
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
  if (query.project_id) {
    params.set("project_id", query.project_id);
  }
  const suffix = params.toString();
  return suffix ? `${path}?${suffix}` : path;
};

export { ApiError };

export const api = {
  getHealth: () => request<HealthResponse>("/healthz"),
  getAuthMe: () => request<AuthMeResponse>("/api/v1/auth/me"),
  listProjects: () => request<ListProjectsResponse>("/api/v1/projects"),
  createProject: (payload: CreateProjectRequest) =>
    request<CreateProjectResponse>("/api/v1/projects", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  listTasks: (query?: TaskListQuery) =>
    request<TaskListResponse>(withSearch("/api/v1/tasks", query)),
  getTaskRunDetail: (runId: string) =>
    request<TaskRunDetail>(`/api/v1/tasks/${encodeURIComponent(runId)}`),
  getTaskEventsUrl: (query?: TaskListQuery) => withSearch("/api/v1/tasks/events", query),
  listSessions: (projectId: string) =>
    request<ListSessionsResponse>(
      projectPath(projectId, "/sessions"),
      withSessionDisplayHostHeader(),
    ),
  loadSubscription: (projectId: string, payload: LoadSubscriptionRequest) =>
    request<LoadSubscriptionResponse>(projectPath(projectId, "/subscriptions/load"), {
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
  syncProxyImports: (payload: SyncProxyImportsRequest) =>
    request<SyncProxyImportsResponse>("/api/v1/proxy-imports/sync", {
      method: "POST",
      body: JSON.stringify(payload),
    }),
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
  getProjectProxySettings: (projectId: string) =>
    request<ProjectProxySettings>(projectPath(projectId, "/proxy-settings")),
  updateProjectProxySettings: (projectId: string, payload: UpdateProjectProxySettingsRequest) =>
    request<ProjectProxySettings>(projectPath(projectId, "/proxy-settings"), {
      method: "PATCH",
      body: JSON.stringify(payload),
    }),
  getSystemSettings: () => request<SystemSettings>("/api/v1/system-settings"),
  updateSystemSettings: (payload: UpdateSystemSettingsRequest) =>
    request<SystemSettings>("/api/v1/system-settings", {
      method: "PATCH",
      body: JSON.stringify(payload),
    }),
  refreshProject: (projectId: string, payload: RefreshRequest) =>
    request<RefreshResponse>(projectPath(projectId, "/refresh"), {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  extractIps: (projectId: string, payload: ExtractIpRequest) =>
    request<ExtractIpResponse>(projectPath(projectId, "/ips/extract"), {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  openSession: (projectId: string, payload: OpenSessionRequest) =>
    request<OpenSessionResponse>(
      projectPath(projectId, "/sessions/open"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  openBatch: (projectId: string, payload: OpenBatchRequest) =>
    request<OpenBatchResponse>(
      projectPath(projectId, "/sessions/open-batch"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  openSessionByNode: (projectId: string, payload: OpenSessionByNodeRequest) =>
    request<OpenSessionResponse>(
      projectPath(projectId, "/sessions/open-by-node"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  openBatchByNode: (projectId: string, payload: OpenBatchByNodeRequest) =>
    request<OpenBatchResponse>(
      projectPath(projectId, "/sessions/open-batch-by-node"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  openSessionByIp: (projectId: string, payload: OpenSessionByIpRequest) =>
    request<OpenSessionResponse>(
      projectPath(projectId, "/sessions/open-by-ip"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  openBatchByIp: (projectId: string, payload: OpenBatchByIpRequest) =>
    request<OpenBatchResponse>(
      projectPath(projectId, "/sessions/open-batch-by-ip"),
      withSessionDisplayHostHeader({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    ),
  updateSessionNode: (projectId: string, sessionId: string, payload: UpdateSessionNodeRequest) =>
    request<OpenSessionResponse>(
      projectPath(projectId, `/sessions/${encodeURIComponent(sessionId)}/node`),
      withSessionDisplayHostHeader({
        method: "PATCH",
        body: JSON.stringify(payload),
      }),
    ),
  getSuggestedPort: (projectId: string) =>
    request<SuggestedPortResponse>(projectPath(projectId, "/sessions/suggested-port")),
  searchSessionOptions: (projectId: string, payload: SearchSessionOptionsRequest) =>
    request<SearchSessionOptionsResponse>(projectPath(projectId, "/ips/options/search"), {
      method: "POST",
      body: JSON.stringify(payload),
    }),
  searchSessionNodeOptions: (
    projectId: string,
    sessionId: string,
    payload: SearchSessionNodeOptionsRequest,
  ) =>
    request<SearchSessionNodeOptionsResponse>(
      projectPath(projectId, `/sessions/${encodeURIComponent(sessionId)}/node-options/search`),
      {
        method: "POST",
        body: JSON.stringify(payload),
      },
    ),
  searchSessionIpNodeOptions: (projectId: string, payload: SearchSessionIpNodeOptionsRequest) =>
    request<SearchSessionIpNodeOptionsResponse>(
      projectPath(projectId, "/sessions/ip-node-options/search"),
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
  closeSession: (projectId: string, sessionId: string) =>
    request<void>(projectPath(projectId, `/sessions/${encodeURIComponent(sessionId)}`), {
      method: "DELETE",
    }),
};
