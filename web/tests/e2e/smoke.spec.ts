import { expect, test } from "@playwright/test";

const RUN_ID_LIVE_SYNC = "run-H6r2Lp8XmQ4Tn7Vc";
const RUN_ID_POST_LOAD = "run-J5w3Ns9Qa1Ze6Ru2";
const RUN_ID_FULL_OK = "run-P4v8Kb2Yt7Lm1Cx5";
const EVENT_ID_LOADING = "evt-C8q3Ls7Vz1Np5Dx9";
const EVENT_ID_PROBING = "evt-F2t6Mw0Rb4Kj8Yu3";
const SESSION_ID_PRIMARY = "sess-A7c2Kp9LmQ4RsT1v";
const SESSION_ID_SECONDARY = "sess-Q8n3Va1Zx5Mw2Lp7";
const GLOBAL_IMPORT_ID = "imp-M7n2Qa8Wx4Rp7Ts1";
const PROFILE_IMPORT_IDS: Record<string, string> = {
  "edge-jp": "imp-V5k3Ld9Hq2Cx8Zm4",
  "fresh-lab": "imp-T8p4Ls2Dw7Hy1Ku6",
};
const CREATED_API_KEY_ID = "key-Q4w8Er2Ty6Ui1Op5";
const CREATED_API_KEY_SECRET = `pbk_${CREATED_API_KEY_ID}_A1b2C3d4E5f6G7h8J9kLm2No`;

const importIdForProfile = (profileId: string) =>
  PROFILE_IMPORT_IDS[profileId] ?? "imp-T8p4Ls2Dw7Hy1Ku6";

const sessionIdFor = (index: 0 | 1) => (index === 0 ? SESSION_ID_PRIMARY : SESSION_ID_SECONDARY);

test.beforeEach(async ({ page }) => {
  const recentTaskBaseSec = Math.floor(Date.now() / 1000) - 120;
  let profiles = ["default", "edge-jp"];
  const profileSettingsByProfile: Record<
    string,
    { profile_id: string; use_global_proxies: boolean }
  > = {
    default: { profile_id: "default", use_global_proxies: true },
    "edge-jp": { profile_id: "edge-jp", use_global_proxies: true },
  };
  let proxyImports: Array<{
    import_id: string;
    name?: string;
    import_kind: "subscription" | "single_node";
    source_scope: { type: "global" } | { type: "profile"; profile_id: string };
    source_identity: { source_type: string; source_value: string };
    allocation_scope: { type: "global" } | { type: "profile"; profile_id: string };
    proxy_count: number;
    distinct_ip_count: number;
    created_at: number;
    updated_at: number;
  }> = [];
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
        run_id: RUN_ID_LIVE_SYNC,
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
        run_id: RUN_ID_POST_LOAD,
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
        run_id: RUN_ID_FULL_OK,
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
        event_id: EVENT_ID_LOADING,
        run_id: RUN_ID_LIVE_SYNC,
        at: recentTaskBaseSec - 9,
        level: "info",
        stage: "loading_subscription",
        message: "Refreshing subscription feed for profile.",
        payload_json: null,
      },
      {
        event_id: EVENT_ID_PROBING,
        run_id: RUN_ID_LIVE_SYNC,
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
        session_id: SESSION_ID_PRIMARY,
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

  const effectiveProfileIdsFor = (item: (typeof proxyImports)[number]) => {
    if (item.allocation_scope.type === "global") {
      return profiles.filter(
        (profileId) => profileSettingsByProfile[profileId]?.use_global_proxies ?? true,
      );
    }
    return [item.allocation_scope.profile_id];
  };

  const proxyImportsResponse = () => ({
    items: proxyImports.map((item) => ({
      ...item,
      effective_profile_ids: effectiveProfileIdsFor(item),
    })),
  });

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
      profileSettingsByProfile[profileId] = {
        profile_id: profileId,
        use_global_proxies: true,
      };
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

  await page.route("**/api/v1/proxies/global/subscriptions/load", async (route) => {
    const payload = JSON.parse(route.request().postData() ?? "{}") as {
      name?: string;
      source?: { type?: string; value?: string };
    };
    proxyImports = [
      {
        import_id: GLOBAL_IMPORT_ID,
        name: payload.name ?? "example.com",
        import_kind: "subscription",
        source_scope: { type: "global" },
        source_identity: {
          source_type: payload.source?.type ?? "url",
          source_value: payload.source?.value ?? "https://example.com/global-subscription.yaml",
        },
        allocation_scope: { type: "global" },
        proxy_count: 12,
        distinct_ip_count: 9,
        created_at: recentTaskBaseSec,
        updated_at: recentTaskBaseSec,
      },
    ];
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        loaded_proxies: 12,
        distinct_ips: 9,
        warnings: [],
      }),
    });
  });

  await page.route("**/api/v1/proxy-imports?*", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(proxyImportsResponse()),
    });
  });

  await page.route("**/api/v1/proxy-imports/*/allocation", async (route) => {
    const importId = route.request().url().split("/").slice(-2, -1)[0] ?? "";
    const payload = JSON.parse(route.request().postData() ?? "{}") as {
      allocation_scope?: { type: "global" } | { type: "profile"; profile_id: string };
    };
    proxyImports = proxyImports.map((item) =>
      item.import_id === importId && payload.allocation_scope
        ? { ...item, allocation_scope: payload.allocation_scope }
        : item,
    );
    await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({}) });
  });

  await page.route("**/api/v1/proxy-imports/*", async (route) => {
    if (route.request().method() === "DELETE") {
      const importId = route.request().url().split("/").pop() ?? "";
      proxyImports = proxyImports.filter((item) => item.import_id !== importId);
      await route.fulfill({ status: 204, body: "" });
      return;
    }
    await route.fallback();
  });

  await page.route("**/api/v1/profiles/*/proxy-settings", async (route) => {
    const profileId = extractProfileId(route.request().url());
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(
          profileSettingsByProfile[profileId] ?? {
            profile_id: profileId,
            use_global_proxies: true,
          },
        ),
      });
      return;
    }

    if (route.request().method() === "PATCH") {
      const payload = JSON.parse(route.request().postData() ?? "{}") as {
        use_global_proxies?: boolean;
      };
      profileSettingsByProfile[profileId] = {
        profile_id: profileId,
        use_global_proxies: payload.use_global_proxies ?? true,
      };
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(profileSettingsByProfile[profileId]),
      });
      return;
    }

    await route.fallback();
  });

  await page.route("**/api/v1/profiles/*/subscriptions/load", async (route) => {
    const profileId = extractProfileId(route.request().url());
    const payload = JSON.parse(route.request().postData() ?? "{}") as {
      name?: string;
      source?: { type?: string; value?: string };
      content?: string;
    };
    proxyImports = [
      ...proxyImports.filter((item) => item.source_scope.type === "global"),
      {
        import_id: importIdForProfile(profileId),
        name: payload.name ?? (payload.content ? `${profileId}-entry` : "example.com"),
        import_kind: payload.content ? "single_node" : "subscription",
        source_scope: { type: "profile", profile_id: profileId },
        source_identity: payload.content
          ? { source_type: "manual", source_value: importIdForProfile(profileId) }
          : {
              source_type: payload.source?.type ?? "url",
              source_value:
                payload.source?.value ?? "https://example.com/profile-subscription.yaml",
            },
        allocation_scope: { type: "profile", profile_id: profileId },
        proxy_count: 48,
        distinct_ip_count: 26,
        created_at: recentTaskBaseSec,
        updated_at: recentTaskBaseSec,
      },
    ];
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

  await page.route("**/api/v1/api-keys*", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          api_keys: [],
        }),
      });
      return;
    }

    if (route.request().method() === "POST") {
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify({
          api_key: {
            key_id: CREATED_API_KEY_ID,
            name: "ops",
            prefix: CREATED_API_KEY_SECRET.slice(0, 18),
            created_by: "dev-admin",
            owner_subject: "dev-admin",
            profile_scope: {
              kind: "selected_profiles",
              profile_ids: ["default"],
            },
            profile_id: "default",
            created_at: 1741748460,
            last_used_at: null,
            revoked_at: null,
          },
          secret: CREATED_API_KEY_SECRET,
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
      session_id: sessionIdFor(0),
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
        session_id: sessionIdFor(0),
        listen: "127.0.0.1:10080",
        port: 10080,
        selected_ip: "203.0.113.10",
        proxy_name: "JP-Tokyo-Entry",
        created_at: 1741748460,
      },
      {
        session_id: sessionIdFor(1),
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

  await page.getByRole("combobox", { name: /config id/i }).click();
  await page.getByPlaceholder("Search configs or type a new ID").fill("edge");
  await page.getByRole("option", { name: /^edge-jp$/i }).click();
  await expect(page.getByRole("combobox", { name: /config id/i })).toContainText("edge-jp");

  await page.getByRole("combobox", { name: /config id/i }).click();
  await page.getByPlaceholder("Search configs or type a new ID").fill("fresh-lab");
  await page.getByText('Create "fresh-lab"').click();
  await expect(page.getByRole("combobox", { name: /config id/i })).toContainText("fresh-lab");

  await page.getByRole("combobox", { name: /config id/i }).click();
  await page.getByText(/^Global$/i).click();
  await page.getByRole("button", { name: /import global pool/i }).click();
  await expect(
    page.getByText("Imported 12 proxies across 9 distinct IPs into the global pool."),
  ).toBeVisible();

  await expect(page.getByText("example.com")).toBeVisible();

  await page.getByRole("combobox", { name: /config id/i }).click();
  await page.getByRole("option", { name: /^fresh-lab$/i }).click();
  await page.getByRole("button", { name: /import local pool/i }).click();
  await expect(
    page.getByText("Imported 48 proxies across 26 distinct IPs into profile fresh-lab."),
  ).toBeVisible();

  await page.getByRole("checkbox", { name: /use global pool for fresh-lab/i }).click();
  await expect(page.getByText("local-only").first()).toBeVisible();

  await page.getByRole("link", { name: /Overview/i }).click();
  await page.getByRole("button", { name: /refresh metadata/i }).click();
  await expect(
    page.getByText("Probed 26 IPs, updated 12 geo records, skipped 14 cached entries."),
  ).toBeVisible();

  await page.getByRole("link", { name: /Tasks/i }).click();
  await expect(page.getByText("Task history and current activity")).toBeVisible();
  await expect(page.getByRole("table").getByText("Subscription sync")).toBeVisible();
  await expect(page.getByText("Refreshing probe metadata.")).toBeVisible();

  await page.getByRole("link", { name: /IP Extract/i }).click();
  await page.getByRole("button", { name: /extract ips/i }).click();
  await expect(page.getByText("203.0.113.10")).toBeVisible();

  await page.getByRole("link", { name: /Sessions/i }).click();
  await page.getByRole("button", { name: /open session/i }).click();
  await expect(
    page.getByText("Listening on 127.0.0.1:10080 via JP-Tokyo-Entry (203.0.113.10)."),
  ).toBeVisible();

  await page.getByRole("tab", { name: /batch open/i }).click();
  await page.getByRole("button", { name: /open batch/i }).click();
  await expect(page.getByText(/Opened 2 sessions in one transaction/)).toBeVisible();

  await page.getByRole("button", { name: /close/i }).first().click();
  await expect(page.getByText(/No active sessions/)).toBeVisible();
});
