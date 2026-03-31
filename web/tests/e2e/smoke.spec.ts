import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  const recentTaskBaseSec = Math.floor(Date.now() / 1000) - 120;
  let profiles = ["default", "edge-jp"];
  const taskList = {
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
        profile_id: "fresh-lab",
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
        profile_id: "fresh-lab",
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
  const taskDetail = {
    run: taskList.runs[0],
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
        payload_json: { targeted_ips: 12 },
      },
    ],
  };
  const sessionsByProfile: Record<
    string,
    Array<{
      session_id: string;
      listen: string;
      port: number;
      selected_ip: string;
      proxy_name: string;
      created_at: number;
    }>
  > = {
    default: [
      {
        session_id: "sess_default_01",
        listen: "127.0.0.1:10080",
        port: 10080,
        selected_ip: "203.0.113.10",
        proxy_name: "JP-Tokyo-Entry",
        created_at: 1741748460,
      },
    ],
    "edge-jp": [],
  };

  const extractProfileId = (url: string) => {
    const pathname = new URL(url).pathname;
    const parts = pathname.split("/");
    return decodeURIComponent(parts[4] ?? "default");
  };

  await page.route("**/healthz", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "ok" }),
    });
  });

  await page.route("**/api/v1/auth/me", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        authenticated: true,
        principal_type: "development",
        subject: "dev-admin",
        email: "dev@example.com",
        groups: ["proxy-broker-admins"],
        is_admin: true,
      }),
    });
  });

  await page.route("**/api/v1/profiles", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ profiles }),
      });
      return;
    }

    if (route.request().method() === "POST") {
      const payload = JSON.parse(route.request().postData() ?? "{}") as { profile_id?: string };
      const profileId = (payload.profile_id ?? "").trim();
      if (!profileId) {
        await route.fulfill({
          status: 400,
          contentType: "application/json",
          body: JSON.stringify({
            code: "invalid_request",
            message: "invalid request: profile_id must not be empty",
          }),
        });
        return;
      }
      if (profiles.includes(profileId)) {
        await route.fulfill({
          status: 409,
          contentType: "application/json",
          body: JSON.stringify({
            code: "profile_exists",
            message: "profile already exists",
          }),
        });
        return;
      }

      profiles = [...profiles, profileId].sort((left, right) => left.localeCompare(right));
      sessionsByProfile[profileId] = [];
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify({ profile_id: profileId }),
      });
      return;
    }

    await route.fallback();
  });

  await page.route("**/api/v1/tasks/events*", async (route) => {
    const snapshotEnvelope = JSON.stringify({
      type: "snapshot",
      data: taskList,
    });
    const summaryEnvelope = JSON.stringify({
      type: "summary",
      data: taskList.summary,
    });
    await route.fulfill({
      status: 200,
      headers: {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
      },
      body: `event: snapshot\ndata: ${snapshotEnvelope}\n\nevent: summary\ndata: ${summaryEnvelope}\n\n`,
    });
  });

  await page.route("**/api/v1/tasks/*", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(taskDetail),
    });
  });

  await page.route("**/api/v1/tasks*", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(taskList),
    });
  });

  await page.route("**/api/v1/profiles/*/subscriptions/load", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        loaded_proxies: 48,
        distinct_ips: 26,
        warnings: ["JP-Relay-02 reused 1 cached IP"],
      }),
    });
  });

  await page.route("**/api/v1/profiles/*/api-keys*", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ api_keys: [] }),
      });
      return;
    }

    if (route.request().method() === "POST") {
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify({
          api_key: {
            key_id: "key_1",
            profile_id: extractProfileId(route.request().url()),
            name: "ops",
            prefix: "pbk_mock",
            created_by: "dev-admin",
            created_at: 1741748460,
            last_used_at: null,
            revoked_at: null,
          },
          secret: "pbk_mock_secret",
        }),
      });
      return;
    }

    if (route.request().method() === "DELETE") {
      await route.fulfill({ status: 204, body: "" });
      return;
    }

    await route.fallback();
  });

  await page.route("**/api/v1/profiles/*/refresh", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ probed_ips: 26, geo_updated: 12, skipped_cached: 14 }),
    });
  });

  await page.route("**/api/v1/profiles/*/ips/extract", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items: [
          {
            ip: "203.0.113.10",
            country_code: "JP",
            country_name: "Japan",
            region_name: "Tokyo",
            city: "Chiyoda",
            probe_ok: true,
            best_latency_ms: 92,
            last_used_at: 1741748460,
          },
        ],
      }),
    });
  });

  await page.route("**/api/v1/profiles/*/ips/options/search", async (route) => {
    const payload = JSON.parse(route.request().postData() ?? "{}") as {
      kind?: "country" | "city" | "ip";
    };
    const items =
      payload.kind === "country"
        ? [{ value: "JP", label: "Japan (JP)", meta: "Japan" }]
        : payload.kind === "city"
          ? [{ value: "JP::Tokyo", label: "Tokyo", meta: "Japan (JP)" }]
          : [{ value: "203.0.113.10", label: "203.0.113.10", meta: "JP / Chiyoda" }];

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ items }),
    });
  });

  await page.route("**/api/v1/profiles/*/nodes/query", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
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
            last_used_at: 1741748460,
            session_count: 1,
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
            session_count: 0,
            subscription_type: "url",
            subscription_value: "https://example.com/subscription.yaml",
          },
        ],
      }),
    });
  });

  await page.route("**/api/v1/profiles/*/nodes/export", async (route) => {
    const payload = (route.request().postDataJSON() ?? {}) as { format?: "csv" | "link_lines" };
    const isCsv = payload.format !== "link_lines";
    await route.fulfill({
      status: 200,
      contentType: isCsv ? "text/csv" : "text/plain",
      body: isCsv
        ? "node_id,proxy_name\nJP-Tokyo-Entry,JP-Tokyo-Entry\n"
        : "vmess://ZXhhbXBsZQ==\n",
    });
  });

  await page.route("**/api/v1/profiles/*/nodes/open-sessions", async (route) => {
    const profileId = extractProfileId(route.request().url());
    const payload = (route.request().postDataJSON() ?? {}) as {
      node_ids?: string[];
      all_filtered?: boolean;
    };
    const availableSessions = [
      {
        session_id: `sess_${profileId}_01`,
        listen: "127.0.0.1:10080",
        port: 10080,
        selected_ip: "203.0.113.10",
        proxy_name: "JP-Tokyo-Entry",
        created_at: 1741748460,
      },
      {
        session_id: `sess_${profileId}_02`,
        listen: "127.0.0.1:10081",
        port: 10081,
        selected_ip: "198.51.100.42",
        proxy_name: "US-SanJose-Fallback",
        created_at: 1741748461,
      },
    ];
    const selectedSessions = payload.all_filtered
      ? availableSessions
      : availableSessions.filter((session) => payload.node_ids?.includes(session.proxy_name));
    sessionsByProfile[profileId] = selectedSessions;
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        sessions: selectedSessions.map(({ created_at: _createdAt, ...session }) => session),
        failures: [],
      }),
    });
  });

  await page.route("**/api/v1/profiles/*/sessions/suggested-port", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ port: 10080 }),
    });
  });

  await page.route("**/api/v1/profiles/*/sessions/open", async (route) => {
    const profileId = extractProfileId(route.request().url());
    const session = {
      session_id: `sess_${profileId}_01`,
      listen: "127.0.0.1:10080",
      port: 10080,
      selected_ip: "203.0.113.10",
      proxy_name: "JP-Tokyo-Entry",
      created_at: 1741748460,
    };
    sessionsByProfile[profileId] = [session];
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        session_id: session.session_id,
        listen: session.listen,
        port: session.port,
        selected_ip: session.selected_ip,
        proxy_name: session.proxy_name,
      }),
    });
  });

  await page.route("**/api/v1/profiles/*/sessions/open-batch", async (route) => {
    const profileId = extractProfileId(route.request().url());
    const sessions = [
      {
        session_id: `sess_${profileId}_01`,
        listen: "127.0.0.1:10080",
        port: 10080,
        selected_ip: "203.0.113.10",
        proxy_name: "JP-Tokyo-Entry",
        created_at: 1741748460,
      },
      {
        session_id: `sess_${profileId}_02`,
        listen: "127.0.0.1:10081",
        port: 10081,
        selected_ip: "203.0.113.88",
        proxy_name: "JP-Osaka-Edge",
        created_at: 1741748461,
      },
    ];
    sessionsByProfile[profileId] = sessions;
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        sessions: sessions.map(({ created_at: _createdAt, ...session }) => session),
      }),
    });
  });

  await page.route("**/api/v1/profiles/*/sessions", async (route) => {
    if (route.request().method() === "GET") {
      const profileId = extractProfileId(route.request().url());
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ sessions: sessionsByProfile[profileId] ?? [] }),
      });
      return;
    }
    await route.fallback();
  });

  await page.route("**/api/v1/profiles/*/sessions/*", async (route) => {
    if (route.request().method() === "DELETE") {
      const profileId = extractProfileId(route.request().url());
      sessionsByProfile[profileId] = [];
      await route.fulfill({ status: 204, body: "" });
      return;
    }
    await route.fallback();
  });
});

test("operator can drive the main workflows", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText(/^Proxy broker$/i)).toBeVisible();
  await expect(page.getByText("Local API heartbeat")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Overview", level: 1 })).toBeVisible();

  await page.getByRole("combobox", { name: /profile id/i }).click();
  await page.getByPlaceholder("Search profiles or type a new ID").fill("edge");
  await page.getByText("edge-jp").click();
  await expect(page.getByRole("combobox", { name: /profile id/i })).toContainText("edge-jp");

  await page.getByRole("combobox", { name: /profile id/i }).click();
  await page.getByPlaceholder("Search profiles or type a new ID").fill("fresh-lab");
  await page.getByText('Create "fresh-lab"').click();
  await expect(page.getByRole("combobox", { name: /profile id/i })).toContainText("fresh-lab");

  await page.getByRole("button", { name: /load subscription/i }).click();
  await expect(page.getByText("Loaded 48 proxies across 26 distinct IPs.")).toBeVisible();

  await page.getByRole("button", { name: /refresh metadata/i }).click();
  await expect(
    page.getByText("Probed 26 IPs, updated 12 geo records, skipped 14 cached entries."),
  ).toBeVisible();

  await page.getByRole("link", { name: /Tasks/i }).click();
  await expect(page.getByText("Task history and current activity")).toBeVisible();
  await expect(page.getByRole("table").getByText("Subscription sync")).toBeVisible();
  await expect(page.getByText("Refreshing probe metadata.")).toBeVisible();

  await page.getByRole("link", { name: /Nodes/i }).click();
  await expect(page.getByRole("heading", { name: "Nodes", level: 1 })).toBeVisible();
  await page.getByRole("tab", { name: /By region/i }).click();
  await expect(page.getByText("JP-Tokyo-Entry")).toBeVisible();
  await page
    .getByRole("combobox")
    .filter({ hasText: /Selected nodes/i })
    .click();
  await page.getByRole("option", { name: /All filtered nodes/i }).click();
  await page.getByRole("button", { name: /^Export$/i }).click();
  await page.getByRole("button", { name: /Node links \(.txt, one per line\)/i }).click();
  await expect(page.getByText("Exported node links as TXT")).toBeVisible();
  await page.getByRole("checkbox").nth(1).click();
  await page
    .getByRole("combobox")
    .filter({ hasText: /All filtered nodes/i })
    .click();
  await page.getByRole("option", { name: /Selected nodes/i }).click();
  await page.getByRole("button", { name: /Create sessions/i }).click();
  await expect(page.getByText("Created 1 sessions")).toBeVisible();

  await page.getByRole("link", { name: /Sessions/i }).click();
  await expect(page.getByText("JP-Tokyo-Entry")).toBeVisible();

  await page.getByRole("tab", { name: /batch open/i }).click();
  await page.getByRole("button", { name: /open batch/i }).click();
  await expect(page.getByText(/Opened 2 sessions in one transaction/)).toBeVisible();

  await page.getByRole("button", { name: /close/i }).first().click();
  await expect(page.getByText(/No active sessions/)).toBeVisible();
});
