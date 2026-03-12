import type {
  ExtractIpResponse,
  HealthResponse,
  ListSessionsResponse,
  LoadSubscriptionResponse,
  OpenBatchResponse,
  OpenSessionResponse,
  RefreshResponse,
} from "@/lib/types";

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
