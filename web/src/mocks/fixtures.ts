import type {
  ExtractIpResponse,
  HealthResponse,
  ListSessionsResponse,
  LoadSubscriptionResponse,
  NodeListResponse,
  NodeOpenSessionsResponse,
  OpenBatchResponse,
  OpenSessionResponse,
  RefreshResponse,
  TaskListResponse,
  TaskRunDetail,
} from "@/lib/types";

const recentTaskBaseSec = Math.floor(Date.now() / 1000) - 120;

export const healthFixture: HealthResponse = {
  status: "ok",
};

export const subscriptionFixture: LoadSubscriptionResponse = {
  loaded_proxies: 48,
  distinct_ips: 26,
  warnings: ["proxy `JP-Relay-02` DNS resolve failed, reused 1 cached ip(s)"],
};

export const refreshFixture: RefreshResponse = {
  probed_ips: 26,
  geo_updated: 12,
  skipped_cached: 14,
};

export const ipResultsFixture: ExtractIpResponse = {
  items: [
    {
      ip: "203.0.113.10",
      country_code: "JP",
      country_name: "Japan",
      region_name: "Tokyo",
      city: "Chiyoda",
      probe_ok: true,
      best_latency_ms: 92,
      last_used_at: 1_741_748_400,
    },
    {
      ip: "198.51.100.42",
      country_code: "US",
      country_name: "United States",
      region_name: "California",
      city: "San Jose",
      probe_ok: false,
      best_latency_ms: null,
      last_used_at: null,
    },
  ],
};

export const nodesFixture: NodeListResponse = {
  total: 3,
  page: 1,
  page_size: 25,
  items: [
    {
      node_id: "JP-Tokyo-Entry",
      proxy_name: "JP-Tokyo-Entry",
      proxy_type: "vmess",
      server: "tokyo-a.example.com",
      preferred_ip: "203.0.113.10",
      ipv4: "203.0.113.10",
      ipv6: "2001:db8::10",
      country_code: "JP",
      country_name: "Japan",
      region_name: "Tokyo",
      city: "Chiyoda",
      probe_status: "reachable",
      best_latency_ms: 92,
      last_used_at: 1_741_748_400,
      session_count: 2,
      subscription_type: "url",
      subscription_value: "https://example.com/subscription.yaml",
    },
    {
      node_id: "JP-Osaka-Edge",
      proxy_name: "JP-Osaka-Edge",
      proxy_type: "trojan",
      server: "osaka-b.example.com",
      preferred_ip: "203.0.113.88",
      ipv4: "203.0.113.88",
      ipv6: null,
      country_code: "JP",
      country_name: "Japan",
      region_name: "Osaka",
      city: "Osaka",
      probe_status: "unreachable",
      best_latency_ms: null,
      last_used_at: null,
      session_count: 0,
      subscription_type: "url",
      subscription_value: "https://example.com/subscription.yaml",
    },
    {
      node_id: "US-SanJose-Fallback",
      proxy_name: "US-SanJose-Fallback",
      proxy_type: "shadowsocks",
      server: "sjc-fallback.example.com",
      preferred_ip: "198.51.100.42",
      ipv4: "198.51.100.42",
      ipv6: null,
      country_code: "US",
      country_name: "United States",
      region_name: "California",
      city: "San Jose",
      probe_status: "unprobed",
      best_latency_ms: null,
      last_used_at: null,
      session_count: 1,
      subscription_type: "url",
      subscription_value: "https://example.com/subscription.yaml",
    },
  ],
};

export const sessionFixture: OpenSessionResponse = {
  session_id: "sess_tokyo_01",
  listen: "127.0.0.1:10080",
  port: 10080,
  selected_ip: "203.0.113.10",
  proxy_name: "JP-Tokyo-Entry",
};

export const batchFixture: OpenBatchResponse = {
  sessions: [
    sessionFixture,
    {
      session_id: "sess_osaka_02",
      listen: "127.0.0.1:10081",
      port: 10081,
      selected_ip: "203.0.113.88",
      proxy_name: "JP-Osaka-Edge",
    },
  ],
};

export const nodeOpenSessionsFixture: NodeOpenSessionsResponse = {
  sessions: [
    sessionFixture,
    {
      session_id: "sess_sanjose_03",
      listen: "127.0.0.1:10082",
      port: 10082,
      selected_ip: "198.51.100.42",
      proxy_name: "US-SanJose-Fallback",
    },
  ],
  failures: [
    {
      node_id: "JP-Osaka-Edge",
      code: "mihomo_unavailable",
      message: "mihomo runtime unavailable: reload timeout",
    },
  ],
};

export const sessionsFixture: ListSessionsResponse = {
  sessions: [
    {
      session_id: "sess_tokyo_01",
      listen: "127.0.0.1:10080",
      port: 10080,
      selected_ip: "203.0.113.10",
      proxy_name: "JP-Tokyo-Entry",
      created_at: 1_741_748_460,
    },
    {
      session_id: "sess_osaka_02",
      listen: "127.0.0.1:10081",
      port: 10081,
      selected_ip: "203.0.113.88",
      proxy_name: "JP-Osaka-Edge",
      created_at: 1_741_748_520,
    },
  ],
};

export const tasksFixture: TaskListResponse = {
  summary: {
    total_runs: 3,
    queued_runs: 1,
    running_runs: 1,
    failed_runs: 0,
    succeeded_runs: 1,
    skipped_runs: 0,
    last_run_at: recentTaskBaseSec,
  },
  runs: [
    {
      run_id: "run_live_sync",
      profile_id: "default",
      kind: "subscription_sync",
      trigger: "schedule",
      status: "running",
      stage: "probing",
      progress_current: 8,
      progress_total: 12,
      created_at: recentTaskBaseSec,
      started_at: recentTaskBaseSec - 10,
      finished_at: null,
      summary_json: null,
      error_code: null,
      error_message: null,
    },
    {
      run_id: "run_post_load",
      profile_id: "default",
      kind: "metadata_refresh_incremental",
      trigger: "post_load",
      status: "queued",
      stage: "queued",
      progress_current: 0,
      progress_total: 6,
      created_at: recentTaskBaseSec - 20,
      started_at: null,
      finished_at: null,
      summary_json: null,
      error_code: null,
      error_message: null,
    },
    {
      run_id: "run_full_ok",
      profile_id: "edge-jp",
      kind: "metadata_refresh_full",
      trigger: "schedule",
      status: "succeeded",
      stage: "completed",
      progress_current: 32,
      progress_total: 32,
      created_at: recentTaskBaseSec - 60,
      started_at: recentTaskBaseSec - 90,
      finished_at: recentTaskBaseSec - 60,
      summary_json: {
        targeted_ips: 32,
        probed_ips: 32,
        geo_updated: 28,
        skipped_cached: 0,
      },
      error_code: null,
      error_message: null,
    },
  ],
  next_cursor: null,
};

export const taskDetailFixture: TaskRunDetail = {
  run: tasksFixture.runs[0] ?? {
    run_id: "run_fallback",
    profile_id: "default",
    kind: "subscription_sync",
    trigger: "schedule",
    status: "queued",
    stage: "queued",
    progress_current: 0,
    progress_total: 0,
    created_at: 0,
    started_at: null,
    finished_at: null,
    summary_json: null,
    error_code: null,
    error_message: null,
  },
  events: [
    {
      event_id: "evt_1",
      run_id: "run_live_sync",
      at: recentTaskBaseSec - 9,
      level: "info",
      stage: "loading_subscription",
      message: "Refreshing subscription feed for profile.",
      payload_json: null,
    },
    {
      event_id: "evt_2",
      run_id: "run_live_sync",
      at: recentTaskBaseSec - 4,
      level: "info",
      stage: "probing",
      message: "Refreshing probe metadata.",
      payload_json: {
        targeted_ips: 12,
      },
    },
  ],
};
