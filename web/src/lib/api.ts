import type {
  ErrorResponse,
  ExtractIpRequest,
  ExtractIpResponse,
  HealthResponse,
  ListSessionsResponse,
  LoadSubscriptionRequest,
  LoadSubscriptionResponse,
  OpenBatchRequest,
  OpenBatchResponse,
  OpenSessionRequest,
  OpenSessionResponse,
  RefreshRequest,
  RefreshResponse,
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

export { ApiError };

export const api = {
  getHealth: () => request<HealthResponse>("/healthz"),
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
  closeSession: (profileId: string, sessionId: string) =>
    request<void>(profilePath(profileId, `/sessions/${encodeURIComponent(sessionId)}`), {
      method: "DELETE",
    }),
};
