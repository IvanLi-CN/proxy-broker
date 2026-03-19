export type SortMode = "mru" | "lru";

export type SubscriptionSource = { type: "url"; value: string } | { type: "file"; value: string };

export interface LoadSubscriptionRequest {
  source: SubscriptionSource;
}

export interface CreateProfileRequest {
  profile_id: string;
}

export interface CreateProfileResponse {
  profile_id: string;
}

export interface LoadSubscriptionResponse {
  loaded_proxies: number;
  distinct_ips: number;
  warnings: string[];
}

export interface RefreshRequest {
  force?: boolean;
}

export interface RefreshResponse {
  probed_ips: number;
  geo_updated: number;
  skipped_cached: number;
}

export interface ExtractIpRequest {
  country_codes?: string[];
  cities?: string[];
  specified_ips?: string[];
  blacklist_ips?: string[];
  limit?: number;
  sort_mode?: SortMode;
}

export interface OpenSessionRequest {
  specified_ip?: string | null;
  selector?: ExtractIpRequest | null;
  desired_port?: number | null;
}

export interface OpenBatchRequest {
  requests: OpenSessionRequest[];
}

export interface OpenSessionResponse {
  session_id: string;
  listen: string;
  port: number;
  selected_ip: string;
  proxy_name: string;
}

export interface OpenBatchResponse {
  sessions: OpenSessionResponse[];
}

export interface ExtractIpItem {
  ip: string;
  country_code?: string | null;
  country_name?: string | null;
  region_name?: string | null;
  city?: string | null;
  probe_ok: boolean;
  best_latency_ms?: number | null;
  last_used_at?: number | null;
}

export interface ExtractIpResponse {
  items: ExtractIpItem[];
}

export interface SessionRecord {
  session_id: string;
  listen: string;
  port: number;
  selected_ip: string;
  proxy_name: string;
  created_at: number;
}

export interface ListSessionsResponse {
  sessions: SessionRecord[];
}

export interface ListProfilesResponse {
  profiles: string[];
}

export interface HealthResponse {
  status: string;
}

export interface ErrorResponse {
  code: string;
  message: string;
  details?: unknown;
}
