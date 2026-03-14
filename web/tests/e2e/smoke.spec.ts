import { expect, test } from "@playwright/test";

const knownProfiles = ["default", "alpha"];

const summaryByProfile: Record<
  string,
  {
    initialized: boolean;
    proxy_count: number;
    distinct_ip_count: number;
    session_count: number;
    probe_ip_count: number;
  }
> = {
  default: {
    initialized: true,
    proxy_count: 48,
    distinct_ip_count: 26,
    session_count: 1,
    probe_ip_count: 26,
  },
  alpha: {
    initialized: true,
    proxy_count: 12,
    distinct_ip_count: 8,
    session_count: 1,
    probe_ip_count: 8,
  },
  "fresh-lab": {
    initialized: false,
    proxy_count: 0,
    distinct_ip_count: 0,
    session_count: 0,
    probe_ip_count: 0,
  },
};

const extractResultsByProfile = {
  default: {
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
  },
  alpha: {
    items: [
      {
        ip: "198.51.100.24",
        country_code: "SG",
        country_name: "Singapore",
        region_name: "Central",
        city: "Singapore",
        probe_ok: true,
        best_latency_ms: 61,
        last_used_at: 1741749400,
      },
    ],
  },
};

test.beforeEach(async ({ page }) => {
  await page.route("**/healthz", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "ok" }),
    });
  });

  const sessionsByProfile: Record<string, Array<Record<string, unknown>>> = {
    default: [
      {
        session_id: "sess_tokyo_01",
        listen: "127.0.0.1:10080",
        port: 10080,
        selected_ip: "203.0.113.10",
        proxy_name: "JP-Tokyo-Entry",
        created_at: 1741748460,
      },
    ],
    alpha: [
      {
        session_id: "sess_alpha_01",
        listen: "127.0.0.1:11080",
        port: 11080,
        selected_ip: "198.51.100.24",
        proxy_name: "SG-Alpha-Entry",
        created_at: 1741749500,
      },
    ],
    "fresh-lab": [],
  };

  await page.route("**/api/v1/profiles", async (route) => {
    const url = new URL(route.request().url());
    if (route.request().method() === "GET" && url.pathname === "/api/v1/profiles") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ profiles: knownProfiles }),
      });
      return;
    }
    await route.fallback();
  });

  await page.route("**/api/v1/profiles/**", async (route) => {
    const url = new URL(route.request().url());
    const [, , , , profileId, ...rest] = url.pathname.split("/");
    const suffix = rest.join("/");

    if (!profileId) {
      await route.fallback();
      return;
    }

    if (route.request().method() === "GET" && suffix === "summary") {
      const summary = summaryByProfile[profileId] ?? {
        initialized: false,
        proxy_count: 0,
        distinct_ip_count: 0,
        session_count: 0,
        probe_ip_count: 0,
      };
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ profile_id: profileId, ...summary }),
      });
      return;
    }

    if (route.request().method() === "POST" && suffix === "subscriptions/load") {
      summaryByProfile[profileId] = {
        initialized: true,
        proxy_count: 48,
        distinct_ip_count: 26,
        session_count: sessionsByProfile[profileId]?.length ?? 0,
        probe_ip_count: 26,
      };
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          loaded_proxies: 48,
          distinct_ips: 26,
          warnings: ["JP-Relay-02 reused 1 cached IP"],
        }),
      });
      return;
    }

    if (route.request().method() === "POST" && suffix === "refresh") {
      const summary = summaryByProfile[profileId] ?? summaryByProfile.default;
      summary.probe_ip_count = Math.max(summary.probe_ip_count, 12);
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          probed_ips: summary.probe_ip_count,
          geo_updated: 12,
          skipped_cached: 14,
        }),
      });
      return;
    }

    if (route.request().method() === "POST" && suffix === "ips/extract") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(
          extractResultsByProfile[profileId as keyof typeof extractResultsByProfile] ?? {
            items: [],
          },
        ),
      });
      return;
    }

    if (route.request().method() === "POST" && suffix === "sessions/open") {
      const created = {
        session_id: `${profileId}_open_01`,
        listen: profileId === "alpha" ? "127.0.0.1:11081" : "127.0.0.1:10080",
        port: profileId === "alpha" ? 11081 : 10080,
        selected_ip: profileId === "alpha" ? "198.51.100.24" : "203.0.113.10",
        proxy_name: profileId === "alpha" ? "SG-Alpha-Entry" : "JP-Tokyo-Entry",
      };
      sessionsByProfile[profileId] = [
        ...(sessionsByProfile[profileId] ?? []),
        { ...created, created_at: 1741749600 },
      ];
      summaryByProfile[profileId] = {
        ...(summaryByProfile[profileId] ?? summaryByProfile.default),
        initialized: true,
        session_count: sessionsByProfile[profileId].length,
      };
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(created),
      });
      return;
    }

    if (route.request().method() === "POST" && suffix === "sessions/open-batch") {
      const created = {
        sessions: [
          {
            session_id: `${profileId}_batch_01`,
            listen: "127.0.0.1:10080",
            port: 10080,
            selected_ip: profileId === "alpha" ? "198.51.100.24" : "203.0.113.10",
            proxy_name: profileId === "alpha" ? "SG-Alpha-Entry" : "JP-Tokyo-Entry",
          },
          {
            session_id: `${profileId}_batch_02`,
            listen: "127.0.0.1:10081",
            port: 10081,
            selected_ip: profileId === "alpha" ? "198.51.100.25" : "203.0.113.88",
            proxy_name: profileId === "alpha" ? "SG-Alpha-Edge" : "JP-Osaka-Edge",
          },
        ],
      };
      sessionsByProfile[profileId] = created.sessions.map((session) => ({
        ...session,
        created_at: 1741749700,
      }));
      summaryByProfile[profileId] = {
        ...(summaryByProfile[profileId] ?? summaryByProfile.default),
        initialized: true,
        session_count: created.sessions.length,
      };
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(created),
      });
      return;
    }

    if (route.request().method() === "GET" && suffix === "sessions") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ sessions: sessionsByProfile[profileId] ?? [] }),
      });
      return;
    }

    if (route.request().method() === "DELETE" && suffix.startsWith("sessions/")) {
      const sessionId = decodeURIComponent(suffix.replace("sessions/", ""));
      sessionsByProfile[profileId] = (sessionsByProfile[profileId] ?? []).filter(
        (session) => session.session_id !== sessionId,
      );
      summaryByProfile[profileId] = {
        ...(summaryByProfile[profileId] ?? summaryByProfile.default),
        session_count: sessionsByProfile[profileId].length,
      };
      await route.fulfill({ status: 204, body: "" });
      return;
    }

    await route.fallback();
  });
});

test("operator can switch project workspaces and keep per-project state", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("Operator control room")).toBeVisible();

  await page.getByRole("button", { name: /load subscription/i }).click();
  await expect(page.getByText("Loaded 48 proxies across 26 distinct IPs.")).toBeVisible();

  await page.getByRole("link", { name: /IP Extract/i }).click();
  await page.getByRole("button", { name: /extract ips/i }).click();
  await expect(page.getByText("203.0.113.10")).toBeVisible();

  await page.getByLabel("Existing projects").click();
  await page.getByRole("option", { name: /alpha/i }).click();
  await expect(page.getByText(/Profile alpha/i)).toBeVisible();

  await page.getByRole("button", { name: /extract ips/i }).click();
  await expect(page.getByText("198.51.100.24")).toBeVisible();
  await expect(page.getByText("203.0.113.10")).not.toBeVisible();

  await page.getByLabel("Existing projects").click();
  await page.getByRole("option", { name: /default/i }).click();
  await expect(page.getByText(/Profile default/i)).toBeVisible();
  await expect(page.getByText("203.0.113.10")).toBeVisible();

  await page.getByRole("link", { name: /Sessions/i }).click();
  await expect(page.getByText("sess_tokyo_01")).toBeVisible();

  await page.getByLabel("New project ID").fill("Fresh Lab");
  await page.getByRole("button", { name: /switch/i }).click();
  await expect(page.getByText(/Profile fresh-lab/i)).toBeVisible();
  await expect(page.getByText("Project not initialized")).toBeVisible();
  await expect(page.getByRole("link", { name: /go load a subscription/i })).toBeVisible();

  await page.getByRole("link", { name: /IP Extract/i }).click();
  await expect(page.getByText("Project not initialized")).toBeVisible();

  await page.getByRole("link", { name: /Overview/i }).click();
  await expect(page.getByText("This project is not initialized yet")).toBeVisible();
});
