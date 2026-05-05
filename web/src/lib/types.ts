export type SortMode = "mru" | "lru";
export type SessionSelectionMode = "any" | "geo" | "ip";
export type SessionOptionKind = "country" | "city" | "ip";

export type SubscriptionSource = { type: "url"; value: string } | { type: "file"; value: string };

export interface LoadSubscriptionRequest {
  name?: string;
  source?: SubscriptionSource;
  content?: string;
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
  resolved_name?: string | null;
  resolved_name_source?:
    | "explicit_input"
    | "existing_import"
    | "parsed_source"
    | "generated"
    | null;
  subscription_metadata?: SubscriptionMetadata | null;
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
  bind_host: string;
  display_host: string;
  display_address: string;
  port: number;
  selected_ip: string;
  proxy_name: string;
  node_id: string;
  candidate_node_ids: string[];
}

export interface OpenBatchResponse {
  sessions: OpenSessionResponse[];
}

export interface OpenSessionByNodeRequest {
  node_id: string;
  desired_port?: number | null;
}

export interface UpdateSessionNodeRequest {
  node_id?: string;
  selected_ip?: string;
  candidate_node_ids?: string[];
}

export interface OpenSessionByIpRequest {
  selected_ip: string;
  candidate_node_ids: string[];
  desired_port?: number | null;
}

export interface OpenBatchByIpRequest {
  requests: OpenSessionByIpRequest[];
}

export interface OpenBatchByNodeItemRequest {
  node_id: string;
  desired_port?: number | null;
}

export interface OpenBatchByNodeRequest {
  node_ids?: string[];
  requests?: OpenBatchByNodeItemRequest[];
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

export type SessionNodeSortMode = "session_recent" | "profile_recent";

export interface SearchSessionNodeOptionsRequest {
  query?: string;
  sort_mode?: SessionNodeSortMode;
  limit?: number;
}

export interface SessionNodeOptionItem {
  node_id: string;
  proxy_name: string;
  import_name?: string | null;
  source_label?: string | null;
  primary_ip?: string | null;
  country_code?: string | null;
  country_name?: string | null;
  region_name?: string | null;
  city?: string | null;
  last_probe_ok?: boolean | null;
  median_latency_ms?: number | null;
  recent_probe_samples: ProxyNodeProbeSampleRecord[];
  session_last_used_at?: number | null;
  profile_last_used_at?: number | null;
}

export interface SearchSessionNodeOptionsResponse {
  items: SessionNodeOptionItem[];
}

export type SessionIpNodeGroupBy = "subscription" | "city";

export interface SearchSessionIpNodeOptionsRequest {
  query?: string;
  group_by?: SessionIpNodeGroupBy;
  session_id?: string;
  limit?: number;
}

export interface SessionIpNodeOptionNodeItem {
  node_id: string;
  proxy_name: string;
  import_name?: string | null;
  source_label?: string | null;
  country_code?: string | null;
  country_name?: string | null;
  region_name?: string | null;
  city?: string | null;
  last_probe_ok?: boolean | null;
  median_latency_ms?: number | null;
  recent_probe_samples: ProxyNodeProbeSampleRecord[];
  profile_last_used_at?: number | null;
  session_last_used_at?: number | null;
}

export interface SessionIpNodeOptionIpItem {
  ip: string;
  group_key: string;
  group_label: string;
  subscription_name?: string | null;
  country_code?: string | null;
  country_name?: string | null;
  region_name?: string | null;
  city?: string | null;
  last_used_at?: number | null;
  best_latency_ms?: number | null;
  nodes: SessionIpNodeOptionNodeItem[];
}

export interface SessionIpNodeOptionGroupItem {
  key: string;
  label: string;
  items: SessionIpNodeOptionIpItem[];
}

export interface SearchSessionIpNodeOptionsResponse {
  groups: SessionIpNodeOptionGroupItem[];
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
  bind_host: string;
  display_host: string;
  display_address: string;
  port: number;
  selected_ip: string;
  proxy_name: string;
  node_id: string;
  candidate_node_ids: string[];
  created_at: number;
}

export interface SessionListItem extends SessionRecord {
  country_code?: string | null;
  country_name?: string | null;
  region_name?: string | null;
  city?: string | null;
}

export interface ListSessionsResponse {
  sessions: SessionListItem[];
}

export interface ListProfilesResponse {
  profiles: string[];
}

export type ProxyScope = { type: "global" } | { type: "profile"; profile_id: string };
export type ProxyImportKind = "subscription" | "single_node";

export interface ProxyImportSourceIdentity {
  source_type: string;
  source_value: string;
}

export interface SubscriptionMetadata {
  source_title?: string | null;
  upload_bytes?: number | null;
  download_bytes?: number | null;
  used_bytes?: number | null;
  total_bytes?: number | null;
  remaining_bytes?: number | null;
  expire_at?: number | null;
}

export interface ProxyImportItem {
  import_id: string;
  name?: string | null;
  import_kind: ProxyImportKind;
  source_scope: ProxyScope;
  source_identity: ProxyImportSourceIdentity;
  allocation_scope: ProxyScope;
  proxy_count: number;
  distinct_ip_count: number;
  effective_profile_ids: string[];
  subscription_metadata?: SubscriptionMetadata | null;
  created_at: number;
  updated_at: number;
}

export interface ListProxyImportResponse {
  items: ProxyImportItem[];
}

export interface ProxyImportListQuery {
  scope?: "all" | "global" | "profile";
  profile_id?: string;
}

export interface UpdateProxyImportAllocationRequest {
  allocation_scope: ProxyScope;
}

export interface ProxyInventoryItem {
  import_id: string;
  node_id: string;
  proxy_name: string;
  proxy_type: string;
  server: string;
  resolved_ips: string[];
  source_scope: ProxyScope;
  allocation_scope: ProxyScope;
  effective_profile_ids: string[];
}

export interface ListProxyInventoryResponse {
  items: ProxyInventoryItem[];
}

export interface ProxyInventoryListQuery {
  scope?: "all" | "global" | "profile";
  profile_id?: string;
}

export interface ProxyNodeMetadataRecord {
  node_id: string;
  ip: string;
  country_code?: string | null;
  country_name?: string | null;
  region_name?: string | null;
  city?: string | null;
  geo_source?: string | null;
  probe_updated_at?: number | null;
  geo_updated_at?: number | null;
  last_probe_ok?: boolean | null;
  last_latency_ms?: number | null;
  median_latency_ms?: number | null;
  last_probe_samples: Array<number | null>;
  recent_probe_samples: ProxyNodeProbeSampleRecord[];
  updated_at: number;
}

export interface ProxyNodeProbeSampleRecord {
  node_id: string;
  ip: string;
  target_url: string;
  ok: boolean;
  latency_ms?: number | null;
  sampled_at: number;
}

export interface ProxyCatalogNodeItem extends ProxyInventoryItem {
  primary_ip?: string | null;
  ip_metadata: ProxyNodeMetadataRecord[];
  can_open_session: boolean;
}

export interface ProxyCatalogGroupItem {
  import: ProxyImportItem;
  nodes: ProxyCatalogNodeItem[];
}

export interface ProxyCatalogResponse {
  view: string;
  profile_id?: string | null;
  groups: ProxyCatalogGroupItem[];
}

export interface ProxyCatalogQuery {
  view?: "global" | "profile";
  profile_id?: string;
}

export interface ProxyOperationRequest {
  view: "global" | "profile";
  profile_id?: string;
  node_ids: string[];
}

export interface ProxyOperationAcceptedResponse {
  run_id: string;
}

export interface UpdateProxyAllocationRequest {
  allocation_scope: ProxyScope;
}

export interface ProfileProxySettings {
  profile_id: string;
  use_global_proxies: boolean;
}

export interface UpdateProfileProxySettingsRequest {
  use_global_proxies: boolean;
}

export interface SystemSettings {
  proxy_probe_interval_sec: number;
  updated_at: number;
}

export interface UpdateSystemSettingsRequest {
  proxy_probe_interval_sec: number;
}

export interface HealthResponse {
  status: string;
}

export type AuthPrincipalType = "human" | "api_key" | "development";

export type ApiKeyProfileScopeKind = "selected_profiles" | "all_profiles";

export interface ApiKeyProfileScope {
  kind: ApiKeyProfileScopeKind;
  profile_ids?: string[];
}

export interface AuthMeResponse {
  authenticated: boolean;
  principal_type: AuthPrincipalType;
  subject: string;
  email?: string | null;
  groups: string[];
  is_admin: boolean;
  profile_id?: string | null;
  api_key_id?: string | null;
  api_key_owner_subject?: string | null;
  api_key_profile_scope?: ApiKeyProfileScope | null;
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
  profile_scope: ApiKeyProfileScope;
}

export interface ApiKeySummary {
  key_id: string;
  name: string;
  prefix: string;
  created_by: string;
  owner_subject: string;
  profile_scope: ApiKeyProfileScope;
  profile_id?: string | null;
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
  | "metadata_refresh_full"
  | "proxy_metadata_refresh"
  | "proxy_latency_probe";

export type TaskRunTrigger = "schedule" | "post_load" | "operator";

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
