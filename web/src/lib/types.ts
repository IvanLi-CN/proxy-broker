export type SortMode = "mru" | "lru";
export type SessionSelectionMode = "any" | "geo" | "ip";
export type SessionOptionKind = "country" | "city" | "ip";
export type NodeProbeStatusFilter = "any" | "reachable" | "unreachable" | "unprobed";
export type NodeSessionPresenceFilter = "any" | "with_sessions" | "without_sessions";
export type NodeIpFamilyFilter = "any" | "ipv4" | "ipv6" | "dual_stack";
export type NodeSortField =
  | "proxy_name"
  | "proxy_type"
  | "preferred_ip"
  | "region"
  | "latency"
  | "last_used_at"
  | "session_count";
export type SortOrder = "asc" | "desc";
export type NodeProbeStatus = "reachable" | "unreachable" | "unprobed";
export type NodeViewMode = "flat" | "group_by_ip" | "group_by_region" | "group_by_subscription";
export type IpFamilyPriority = "ipv4_first";
export type NodeExportFormat = "csv" | "link_lines";

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
  selection_mode: SessionSelectionMode;
  country_codes?: string[];
  cities?: string[];
  specified_ips?: string[];
  excluded_ips?: string[];
  sort_mode?: SortMode;
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

export interface SuggestedPortResponse {
  port: number;
}

export interface SearchSessionOptionsRequest {
  kind: SessionOptionKind;
  query?: string;
  country_codes?: string[];
  cities?: string[];
  limit?: number;
}

export interface SessionOptionItem {
  value: string;
  label: string;
  meta?: string | null;
}

export interface SearchSessionOptionsResponse {
  items: SessionOptionItem[];
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

export interface NodeListQuery {
  query?: string;
  proxy_types?: string[];
  country_codes?: string[];
  regions?: string[];
  cities?: string[];
  probe_status?: NodeProbeStatusFilter;
  session_presence?: NodeSessionPresenceFilter;
  ip_family?: NodeIpFamilyFilter;
  sort_by?: NodeSortField;
  sort_order?: SortOrder;
  page?: number;
  page_size?: number;
}

export interface NodeListItem {
  node_id: string;
  proxy_name: string;
  proxy_type: string;
  server: string;
  preferred_ip?: string | null;
  ipv4?: string | null;
  ipv6?: string | null;
  country_code?: string | null;
  country_name?: string | null;
  region_name?: string | null;
  city?: string | null;
  probe_status: NodeProbeStatus;
  best_latency_ms?: number | null;
  last_used_at?: number | null;
  session_count: number;
  subscription_type: string;
  subscription_value: string;
}

export interface NodeListResponse {
  items: NodeListItem[];
  total: number;
  page: number;
  page_size: number;
}

export interface NodeExportRequest {
  node_ids?: string[];
  all_filtered?: boolean;
  query?: NodeListQuery;
  format?: NodeExportFormat;
}

export interface NodeOpenSessionsRequest {
  node_ids?: string[];
  all_filtered?: boolean;
  query?: NodeListQuery;
  ip_family_priority?: IpFamilyPriority;
}

export interface NodeOpenSessionFailure {
  node_id: string;
  code: string;
  message: string;
}

export interface NodeOpenSessionsResponse {
  sessions: OpenSessionResponse[];
  failures: NodeOpenSessionFailure[];
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

export type AuthPrincipalType = "human" | "api_key" | "development";

export interface AuthMeResponse {
  authenticated: boolean;
  principal_type: AuthPrincipalType;
  subject: string;
  email?: string | null;
  groups: string[];
  is_admin: boolean;
  profile_id?: string | null;
  api_key_id?: string | null;
}

export type CurrentUserState =
  | {
      status: "loading";
    }
  | {
      status: "anonymous";
    }
  | {
      status: "error";
      message: string;
    }
  | {
      status: "resolved";
      identity: AuthMeResponse;
    };

export interface CreateApiKeyRequest {
  name: string;
}

export interface ApiKeySummary {
  key_id: string;
  profile_id: string;
  name: string;
  prefix: string;
  created_by: string;
  created_at: number;
  last_used_at?: number | null;
  revoked_at?: number | null;
}

export interface ListApiKeysResponse {
  api_keys: ApiKeySummary[];
}

export interface CreateApiKeyResponse {
  api_key: ApiKeySummary;
  secret: string;
}

export interface ErrorResponse {
  code: string;
  message: string;
  details?: unknown;
}

export type TaskRunKind =
  | "subscription_sync"
  | "metadata_refresh_incremental"
  | "metadata_refresh_full";

export type TaskRunTrigger = "schedule" | "post_load";

export type TaskRunStatus = "queued" | "running" | "succeeded" | "failed" | "skipped";

export type TaskRunStage =
  | "queued"
  | "loading_subscription"
  | "diffing_inventory"
  | "probing"
  | "geo_enrichment"
  | "persisting"
  | "completed";

export type TaskEventLevel = "info" | "warning" | "error";

export interface TaskRunSummary {
  run_id: string;
  profile_id: string;
  kind: TaskRunKind;
  trigger: TaskRunTrigger;
  status: TaskRunStatus;
  stage: TaskRunStage;
  progress_current?: number | null;
  progress_total?: number | null;
  created_at: number;
  started_at?: number | null;
  finished_at?: number | null;
  summary_json?: Record<string, unknown> | null;
  error_code?: string | null;
  error_message?: string | null;
}

export interface TaskRunEvent {
  event_id: string;
  run_id: string;
  at: number;
  level: TaskEventLevel;
  stage: TaskRunStage;
  message: string;
  payload_json?: Record<string, unknown> | null;
}

export interface TaskSummary {
  total_runs: number;
  queued_runs: number;
  running_runs: number;
  failed_runs: number;
  succeeded_runs: number;
  skipped_runs: number;
  last_run_at?: number | null;
}

export interface TaskListQuery {
  profile_id?: string;
  kind?: TaskRunKind;
  status?: TaskRunStatus;
  trigger?: TaskRunTrigger;
  running_only?: boolean;
  since?: number;
  limit?: number;
  cursor?: string;
}

export interface TaskListResponse {
  summary: TaskSummary;
  runs: TaskRunSummary[];
  next_cursor?: string | null;
}

export interface TaskRunDetail {
  run: TaskRunSummary;
  events: TaskRunEvent[];
}

export interface TaskStreamEnvelope<T = unknown> {
  type: "snapshot" | "run-upsert" | "run-event" | "summary" | "heartbeat";
  data: T;
}
