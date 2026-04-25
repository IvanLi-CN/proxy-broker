import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "@/lib/api";

describe("session display host header wiring", () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    window.history.replaceState({}, "", "/sessions");
  });

  afterEach(() => {
    vi.restoreAllMocks();
    globalThis.fetch = originalFetch;
  });

  it("keeps the JSON content type when adding the display-host header", async () => {
    const fetchMock = vi.fn(async (..._args: [RequestInfo | URL, RequestInit?]) => ({
      ok: true,
      json: async () => ({
        session_id: "sess-123",
        listen: "127.0.0.1:10080",
        bind_host: "127.0.0.1",
        display_host: "panel.example.test",
        display_address: "panel.example.test:10080",
        port: 10080,
        selected_ip: "203.0.113.10",
        proxy_name: "JP-Tokyo-Entry",
        node_id: "node-jp-tokyo-entry",
      }),
    }));
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    await api.openSession("default", {
      selection_mode: "any",
      country_codes: [],
      cities: [],
      specified_ips: [],
      excluded_ips: [],
      sort_mode: "lru",
      desired_port: 10080,
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const firstCall = fetchMock.mock.calls.at(0);
    const init = firstCall?.[1];
    expect(init).toMatchObject({
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Proxy-Broker-Display-Host": window.location.hostname,
      },
    });
  });
});
