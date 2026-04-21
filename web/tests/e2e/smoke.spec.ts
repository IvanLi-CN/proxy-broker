import { expect, test } from "@playwright/test";

const RUN_ID_LIVE_SYNC = "run-H6r2Lp8XmQ4Tn7Vc";
const RUN_ID_POST_LOAD = "run-J5w3Ns9Qa1Ze6Ru2";
const RUN_ID_FULL_OK = "run-P4v8Kb2Yt7Lm1Cx5";
const RUN_ID_PROXY_PROBE = "run-ProxyProbeFreshLab";
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
const FRESH_LAB_NODE_IDS = ["node-fresh-lab-01", "node-fresh-lab-02"] as const;

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
      node_id?: string;
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

  const localNodesByProfile: Record<
    string,
    Array<{
      node_id: string;
      proxy_name: string;
      proxy_type: string;
      server: string;
      resolved_ips: string[];
      primary_ip: string;
      ip_metadata: Array<{
        node_id: string;
        ip: string;
        country_code: string;
        country_name: string;
        region_name: string;
        city: string;
        geo_source: string;
        probe_updated_at: number;
        geo_updated_at: number;
        last_probe_ok: boolean | null;
        last_latency_ms: number | null;
        median_latency_ms: number | null;
        last_probe_samples: Array<number | null>;
        updated_at: number;
      }>;
    }>
  > = {
    "fresh-lab": [
      {
        node_id: FRESH_LAB_NODE_IDS[0],
        proxy_name: "Fresh-Lab-01",
        proxy_type: "vmess",
        server: "fresh-lab-a.example.com:443",
        resolved_ips: ["203.0.113.101"],
        primary_ip: "203.0.113.101",
        ip_metadata: [
          {
            node_id: FRESH_LAB_NODE_IDS[0],
            ip: "203.0.113.101",
            country_code: "JP",
            country_name: "Japan",
            region_name: "Tokyo",
            city: "Shibuya",
            geo_source: "geoip",
            probe_updated_at: recentTaskBaseSec,
            geo_updated_at: recentTaskBaseSec,
            last_probe_ok: true,
            last_latency_ms: 104,
            median_latency_ms: 106,
            last_probe_samples: [108, 106, 104, 110, 107],
            updated_at: recentTaskBaseSec,
          },
        ],
      },
      {
        node_id: FRESH_LAB_NODE_IDS[1],
        proxy_name: "Fresh-Lab-02",
        proxy_type: "trojan",
        server: "fresh-lab-b.example.com:443",
        resolved_ips: ["203.0.113.102"],
        primary_ip: "203.0.113.102",
        ip_metadata: [
          {
            node_id: FRESH_LAB_NODE_IDS[1],
            ip: "203.0.113.102",
            country_code: "JP",
            country_name: "Japan",
            region_name: "Osaka",
            city: "Osaka",
            geo_source: "geoip",
            probe_updated_at: recentTaskBaseSec,
            geo_updated_at: recentTaskBaseSec,
            last_probe_ok: null,
            last_latency_ms: null,
            median_latency_ms: null,
            last_probe_samples: [],
            updated_at: recentTaskBaseSec,
          },
        ],
      },
    ],
    "edge-jp": [
      {
        node_id: "node-edge-jp-01",
        proxy_name: "Edge-JP-01",
        proxy_type: "ss",
        server: "edge-jp.example.com:8443",
        resolved_ips: ["198.51.100.42"],
        primary_ip: "198.51.100.42",
        ip_metadata: [],
      },
    ],
  };

  const globalNodes = (
    allocationScope: { type: "global" } | { type: "profile"; profile_id: string },
  ) => [
    {
      node_id: "node-global-jp-tokyo",
      proxy_name: "JP-Tokyo-Entry",
      proxy_type: "vmess",
      server: "tokyo-a.example.com:443",
      resolved_ips: ["203.0.113.10"],
      primary_ip: "203.0.113.10",
      allocation_scope: allocationScope,
      ip_metadata: [
        {
          node_id: "node-global-jp-tokyo",
          ip: "203.0.113.10",
          country_code: "JP",
          country_name: "Japan",
          region_name: "Tokyo",
          city: "Chiyoda",
          geo_source: "geoip",
          probe_updated_at: recentTaskBaseSec,
          geo_updated_at: recentTaskBaseSec,
          last_probe_ok: true,
          last_latency_ms: 92,
          median_latency_ms: 92,
          last_probe_samples: [90, 88, 92, 95, 91],
          updated_at: recentTaskBaseSec,
        },
      ],
    },
    {
      node_id: "node-global-jp-osaka",
      proxy_name: "JP-Osaka-Edge",
      proxy_type: "trojan",
      server: "osaka-b.example.com:443",
      resolved_ips: ["203.0.113.88"],
      primary_ip: "203.0.113.88",
      allocation_scope: allocationScope,
      ip_metadata: [
        {
          node_id: "node-global-jp-osaka",
          ip: "203.0.113.88",
          country_code: "JP",
          country_name: "Japan",
          region_name: "Osaka",
          city: "Osaka",
          geo_source: "geoip",
          probe_updated_at: recentTaskBaseSec,
          geo_updated_at: recentTaskBaseSec,
          last_probe_ok: false,
          last_latency_ms: null,
          median_latency_ms: null,
          last_probe_samples: [null, null, null, null, null],
          updated_at: recentTaskBaseSec,
        },
      ],
    },
  ];

  const proxyCatalogResponse = (view: "global" | "profile", requestedProfileId?: string) => {
    const imports = proxyImportsResponse().items.filter((item) =>
      view === "global"
        ? true
        : Boolean(requestedProfileId && item.effective_profile_ids.includes(requestedProfileId)),
    );

    return {
      view,
      profile_id: view === "profile" ? (requestedProfileId ?? null) : null,
      groups: imports.map((item) => {
        const baseNodes =
          item.import_id === GLOBAL_IMPORT_ID
            ? globalNodes(item.allocation_scope)
            : (localNodesByProfile[
                item.source_scope.type === "profile" ? item.source_scope.profile_id : ""
              ] ?? []);

        return {
          import: item,
          nodes: baseNodes.map((node) => ({
            import_id: item.import_id,
            node_id: node.node_id,
            proxy_name: node.proxy_name,
            proxy_type: node.proxy_type,
            server: node.server,
            resolved_ips: node.resolved_ips,
            source_scope: item.source_scope,
            allocation_scope:
              "allocation_scope" in node ? node.allocation_scope : item.allocation_scope,
            effective_profile_ids: item.effective_profile_ids,
            primary_ip: node.primary_ip,
            ip_metadata: node.ip_metadata,
            can_open_session: view === "profile",
          })),
        };
      }),
    };
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
    const profileId = new URL(route.request().url()).searchParams.get("profile_id");
    const proxyProbeRun = {
      run_id: RUN_ID_PROXY_PROBE,
      profile_id: "fresh-lab",
      kind: "proxy_latency_probe",
      trigger: "operator",
      status: "running",
      stage: "probing",
      progress_current: 3,
      progress_total: 5,
      created_at: recentTaskBaseSec - 5,
      started_at: recentTaskBaseSec - 4,
      finished_at: null,
      summary_json: null,
      error_code: null,
      error_message: null,
    };
    const scopedTaskList =
      profileId === "fresh-lab"
        ? {
            ...taskList,
            summary: {
              ...taskList.summary,
              total_runs: taskList.summary.total_runs + 1,
              running_runs: taskList.summary.running_runs + 1,
            },
            runs: [proxyProbeRun, ...taskList.runs],
          }
        : taskList;
    const envelopes = [
      `event: snapshot\ndata: ${JSON.stringify({ type: "snapshot", data: scopedTaskList })}\n\n`,
      `event: summary\ndata: ${JSON.stringify({ type: "summary", data: scopedTaskList.summary })}\n\n`,
    ];
    if (profileId === "fresh-lab") {
      envelopes.push(
        `event: run-upsert\ndata: ${JSON.stringify({ type: "run-upsert", data: proxyProbeRun })}\n\n`,
      );
      envelopes.push(
        `event: run-event\ndata: ${JSON.stringify({
          type: "run-event",
          data: {
            event_id: "evt-proxy-probe-fresh-lab",
            run_id: RUN_ID_PROXY_PROBE,
            at: recentTaskBaseSec - 1,
            level: "info",
            stage: "probing",
            message: "probe round 3 timeout",
            payload_json: {
              node_id: FRESH_LAB_NODE_IDS[0],
              round: 3,
              sample_ms: null,
              samples_total: 5,
              progress_current: 3,
              progress_total: 5,
            },
          },
        })}\n\n`,
      );
    }
    await route.fulfill({
      status: 200,
      headers: {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
      },
      body: envelopes.join(""),
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
        name: payload.content ? `${profileId}-entry` : `${profileId}-local`,
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

  await page.route("**/api/v1/proxy-catalog?*", async (route) => {
    const url = new URL(route.request().url());
    const view = (url.searchParams.get("view") ?? "global") as "global" | "profile";
    const profileId = url.searchParams.get("profile_id") ?? undefined;
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(proxyCatalogResponse(view, profileId)),
    });
  });

  await page.route("**/api/v1/proxy-ops/refresh", async (route) => {
    await route.fulfill({
      status: 202,
      contentType: "application/json",
      body: JSON.stringify({ run_id: "run-ProxyRefreshQueued" }),
    });
  });

  await page.route("**/api/v1/proxy-ops/probe", async (route) => {
    await route.fulfill({
      status: 202,
      contentType: "application/json",
      body: JSON.stringify({ run_id: RUN_ID_PROXY_PROBE }),
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

  await page.route("**/api/v1/profiles/*/sessions/open-by-node", async (route) => {
    const profileId = extractProfileId(route.request().url());
    const payload = JSON.parse(route.request().postData() ?? "{}") as {
      node_id?: string;
      desired_port?: number;
    };
    const nodeId = payload.node_id ?? FRESH_LAB_NODE_IDS[0];
    const nodeMap: Record<
      string,
      { proxy_name: string; selected_ip: string; listen: string; port: number }
    > = {
      [FRESH_LAB_NODE_IDS[0]]: {
        proxy_name: "Fresh-Lab-01",
        selected_ip: "203.0.113.101",
        listen: "127.0.0.1:10080",
        port: 10080,
      },
      [FRESH_LAB_NODE_IDS[1]]: {
        proxy_name: "Fresh-Lab-02",
        selected_ip: "203.0.113.102",
        listen: "127.0.0.1:10081",
        port: 10081,
      },
    };
    const sessionShape = nodeMap[nodeId] ?? nodeMap[FRESH_LAB_NODE_IDS[0]];
    const session = {
      session_id: sessionIdFor(0),
      ...sessionShape,
      port: payload.desired_port ?? sessionShape.port,
      listen: `127.0.0.1:${payload.desired_port ?? sessionShape.port}`,
      node_id: nodeId,
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
        node_id: session.node_id,
      }),
    });
  });

  await page.route("**/api/v1/profiles/*/sessions/open-batch-by-node", async (route) => {
    const profileId = extractProfileId(route.request().url());
    const payload = JSON.parse(route.request().postData() ?? "{}") as {
      node_ids?: string[];
      requests?: Array<{ node_id: string; desired_port?: number }>;
    };
    const requests =
      payload.requests ??
      (payload.node_ids ?? [...FRESH_LAB_NODE_IDS]).map((nodeId) => ({ node_id: nodeId }));
    const sessions = requests.map((request, index) => ({
      session_id: sessionIdFor(index === 0 ? 0 : 1),
      listen: `127.0.0.1:${request.desired_port ?? (index === 0 ? 10080 : 10081)}`,
      port: request.desired_port ?? (index === 0 ? 10080 : 10081),
      selected_ip: index === 0 ? "203.0.113.101" : "203.0.113.102",
      proxy_name: index === 0 ? "Fresh-Lab-01" : "Fresh-Lab-02",
      node_id: request.node_id,
      created_at: 1741748460 + index,
    }));
    sessionsByProfile[profileId] = sessions;
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        sessions: sessions.map(({ created_at: _createdAt, ...session }) => session),
      }),
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
      node_id: "node-global-jp-tokyo",
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
        node_id: session.node_id,
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
        node_id: "node-global-jp-tokyo",
        created_at: 1741748460,
      },
      {
        session_id: sessionIdFor(1),
        listen: "127.0.0.1:10081",
        port: 10081,
        selected_ip: "203.0.113.88",
        proxy_name: "JP-Osaka-Edge",
        node_id: "node-global-jp-osaka",
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

  await expect(page.getByText("example.com", { exact: true })).toBeVisible();

  await page.getByRole("combobox", { name: /config id/i }).click();
  await page.getByRole("option", { name: /^fresh-lab$/i }).click();
  await page.getByRole("button", { name: /import local pool/i }).click();
  await expect(
    page.getByText("Imported 48 proxies across 26 distinct IPs into profile fresh-lab."),
  ).toBeVisible();

  await page.getByRole("checkbox", { name: /use global pool for fresh-lab/i }).click();
  await expect(page.getByText("local-only").first()).toBeVisible();
  await expect(page.getByText("Current profile grouped nodes", { exact: true })).toBeVisible();
  await expect(page.getByText("Fresh-Lab-01")).toBeVisible();
  await expect(page.getByText(/Live stream:/i)).toBeVisible();
  await page.getByRole("checkbox", { name: /select import group fresh-lab-local/i }).click();
  await page.getByRole("button", { name: /probe selected/i }).click();
  await expect(page.getByText("Queued latency probe")).toBeVisible();
  await expect(page.getByText(`Run ID: ${RUN_ID_PROXY_PROBE}`)).toBeVisible();
  await page.getByRole("button", { name: /create sessions/i }).click();
  await expect(page.getByText("Create node-pinned sessions")).toBeVisible();
  const desiredPortInputs = page.getByLabel("Desired port (optional)");
  await desiredPortInputs.first().fill("10080");
  await desiredPortInputs.nth(1).fill("10081");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: /^create sessions$/i })
    .click();
  await expect(page.getByText("Opened 2 sessions in batch")).toBeVisible();

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
  await expect(page.getByText("Fresh-Lab-01")).toBeVisible();
  await expect(page.getByText("Fresh-Lab-02")).toBeVisible();

  await page.getByRole("button", { name: /close/i }).first().click();
  await expect(page.getByText(/No active sessions/)).toBeVisible();
});
