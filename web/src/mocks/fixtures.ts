import type {
  ExtractIpResponse,
  HealthResponse,
  ListSessionsResponse,
  LoadSubscriptionResponse,
  OpenBatchResponse,
  OpenSessionResponse,
  RefreshResponse,
  SearchSessionNodeOptionsResponse,
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
  resolved_name: "edge-feed",
  resolved_name_source: "parsed_source",
  subscription_metadata: {
    source_title: "edge-feed",
    upload_bytes: 10 * 1024 ** 3,
    download_bytes: 20 * 1024 ** 3,
    used_bytes: 30 * 1024 ** 3,
    total_bytes: 100 * 1024 ** 3,
    remaining_bytes: 70 * 1024 ** 3,
    expire_at: 1_741_748_800,
  },
  warnings: [
    "proxy `JP-Relay-02` DNS resolve failed, reused 1 cached ip(s)",
    "filtered informational subscription entry `剩余流量 70GB`",
  ],
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

export const sessionFixture: OpenSessionResponse = {
  session_id: "sess-A7c2Kp9LmQ4RsT1v",
  listen: "127.0.0.1:10080",
  bind_host: "127.0.0.1",
  display_host: "127.0.0.1",
  display_address: "127.0.0.1:10080",
  port: 10080,
  selected_ip: "203.0.113.10",
  proxy_name: "JP-Tokyo-Entry",
  node_id: "node-jp-tokyo-entry",
};

export const batchFixture: OpenBatchResponse = {
  sessions: [
    sessionFixture,
    {
      session_id: "sess-Q8n3Va1Zx5Mw2Lp7",
      listen: "127.0.0.1:10081",
      bind_host: "127.0.0.1",
      display_host: "127.0.0.1",
      display_address: "127.0.0.1:10081",
      port: 10081,
      selected_ip: "203.0.113.88",
      proxy_name: "JP-Osaka-Edge",
      node_id: "node-jp-osaka-edge",
    },
  ],
};

export const sessionsFixture: ListSessionsResponse = {
  sessions: [
    {
      session_id: "sess-A7c2Kp9LmQ4RsT1v",
      listen: "127.0.0.1:10080",
      bind_host: "127.0.0.1",
      display_host: "127.0.0.1",
      display_address: "127.0.0.1:10080",
      port: 10080,
      selected_ip: "203.0.113.10",
      proxy_name: "JP-Tokyo-Entry",
      node_id: "node-jp-tokyo-entry",
      created_at: 1_741_748_460,
      country_code: "JP",
      country_name: "Japan",
      region_name: "Tokyo",
      city: "Chiyoda",
    },
    {
      session_id: "sess-Q8n3Va1Zx5Mw2Lp7",
      listen: "127.0.0.1:10081",
      bind_host: "127.0.0.1",
      display_host: "127.0.0.1",
      display_address: "127.0.0.1:10081",
      port: 10081,
      selected_ip: "203.0.113.88",
      proxy_name: "JP-Osaka-Edge",
      node_id: "node-jp-osaka-edge",
      created_at: 1_741_748_520,
      country_code: "JP",
      country_name: "Japan",
      region_name: "Osaka",
      city: "Osaka",
    },
  ],
};

export const sessionNodeOptionsFixture: SearchSessionNodeOptionsResponse = {
  items: [
    {
      node_id: "node-jp-tokyo-entry",
      proxy_name: "JP-Tokyo-Entry",
      import_name: "browser-core",
      source_label: "browser-core",
      primary_ip: "203.0.113.10",
      country_code: "JP",
      country_name: "Japan",
      region_name: "Tokyo",
      city: "Chiyoda",
      last_probe_ok: true,
      median_latency_ms: 88,
      session_last_used_at: 1_741_748_520,
      profile_last_used_at: 1_741_748_520,
    },
    {
      node_id: "node-jp-osaka-edge",
      proxy_name: "JP-Osaka-Edge",
      import_name: "browser-core",
      source_label: "browser-core",
      primary_ip: "203.0.113.88",
      country_code: "JP",
      country_name: "Japan",
      region_name: "Osaka",
      city: "Osaka",
      last_probe_ok: true,
      median_latency_ms: 103,
      session_last_used_at: 1_741_748_200,
      profile_last_used_at: 1_741_748_460,
    },
    {
      node_id: "node-us-sanjose-edge",
      proxy_name: "US-SanJose-Edge",
      import_name: "fallback-lab",
      source_label: "fallback-lab",
      primary_ip: "198.51.100.42",
      country_code: "US",
      country_name: "United States",
      region_name: "California",
      city: "San Jose",
      last_probe_ok: false,
      median_latency_ms: null,
      session_last_used_at: null,
      profile_last_used_at: 1_741_747_900,
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
      run_id: "run-H6r2Lp8XmQ4Tn7Vc",
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
      run_id: "run-J5w3Ns9Qa1Ze6Ru2",
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
      run_id: "run-P4v8Kb2Yt7Lm1Cx5",
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
    run_id: "run-R6m2Hd8Wp3Qs9Ty4",
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
      event_id: "evt-C8q3Ls7Vz1Np5Dx9",
      run_id: "run-H6r2Lp8XmQ4Tn7Vc",
      at: recentTaskBaseSec - 9,
      level: "info",
      stage: "loading_subscription",
      message: "Refreshing subscription feed for profile.",
      payload_json: null,
    },
    {
      event_id: "evt-F2t6Mw0Rb4Kj8Yu3",
      run_id: "run-H6r2Lp8XmQ4Tn7Vc",
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
