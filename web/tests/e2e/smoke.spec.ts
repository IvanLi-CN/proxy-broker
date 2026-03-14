import { expect, test } from "@playwright/test";

const profile = "default";

test.beforeEach(async ({ page }) => {
  await page.route("**/healthz", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "ok" }),
    });
  });

  await page.route(`**/api/v1/profiles/${profile}/subscriptions/load`, async (route) => {
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

  await page.route(`**/api/v1/profiles/${profile}/refresh`, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ probed_ips: 26, geo_updated: 12, skipped_cached: 14 }),
    });
  });

  await page.route(`**/api/v1/profiles/${profile}/ips/extract`, async (route) => {
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

  await page.route(`**/api/v1/profiles/${profile}/sessions/open`, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        session_id: "sess_tokyo_01",
        listen: "127.0.0.1:10080",
        port: 10080,
        selected_ip: "203.0.113.10",
        proxy_name: "JP-Tokyo-Entry",
      }),
    });
  });

  await page.route(`**/api/v1/profiles/${profile}/sessions/open-batch`, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        sessions: [
          {
            session_id: "sess_tokyo_01",
            listen: "127.0.0.1:10080",
            port: 10080,
            selected_ip: "203.0.113.10",
            proxy_name: "JP-Tokyo-Entry",
          },
          {
            session_id: "sess_osaka_02",
            listen: "127.0.0.1:10081",
            port: 10081,
            selected_ip: "203.0.113.88",
            proxy_name: "JP-Osaka-Edge",
          },
        ],
      }),
    });
  });

  let sessions = [
    {
      session_id: "sess_tokyo_01",
      listen: "127.0.0.1:10080",
      port: 10080,
      selected_ip: "203.0.113.10",
      proxy_name: "JP-Tokyo-Entry",
      created_at: 1741748460,
    },
  ];

  await page.route(`**/api/v1/profiles/${profile}/sessions`, async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ sessions }),
      });
      return;
    }
    await route.fallback();
  });

  await page.route(`**/api/v1/profiles/${profile}/sessions/*`, async (route) => {
    if (route.request().method() === "DELETE") {
      sessions = [];
      await route.fulfill({ status: 204, body: "" });
      return;
    }
    await route.fallback();
  });
});

test("operator can drive the main workflows", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("Operator control room")).toBeVisible();
  await expect(page.getByText("Local API heartbeat")).toBeVisible();
  await expect(
    page.getByText("Run the operator plane like a control room, not a note pile."),
  ).toBeVisible();

  await page.getByRole("button", { name: /load subscription/i }).click();
  await expect(page.getByText("Loaded 48 proxies across 26 distinct IPs.")).toBeVisible();

  await page.getByRole("button", { name: /refresh metadata/i }).click();
  await expect(
    page.getByText("Probed 26 IPs, updated 12 geo records, skipped 14 cached entries."),
  ).toBeVisible();

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

  await page.getByRole("button", { name: /close/i }).click();
  await expect(page.getByText(/No active sessions/)).toBeVisible();
});
