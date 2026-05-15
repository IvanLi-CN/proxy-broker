import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { I18nProvider } from "@/i18n";
import type { CurrentUserState, ProxyCatalogResponse } from "@/lib/types";
import { ProxiesPage } from "@/pages/ProxiesPage";

const currentUser: CurrentUserState = {
  status: "resolved",
  identity: {
    authenticated: true,
    principal_type: "human",
    subject: "admin@example.com",
    email: "admin@example.com",
    groups: ["admin"],
    is_admin: true,
  },
};

const nonAdminUser: CurrentUserState = {
  status: "resolved",
  identity: {
    authenticated: true,
    principal_type: "human",
    subject: "user@example.com",
    email: "user@example.com",
    groups: [],
    is_admin: false,
  },
};

function createProbeSamples(nodeId: string, ip: string, samples: Array<number | null>) {
  return samples.map((latencyMs, index) => ({
    node_id: nodeId,
    ip,
    target_url: "https://www.gstatic.com/generate_204",
    ok: latencyMs != null,
    latency_ms: latencyMs,
    sampled_at: 1_713_309_380 - index * 20,
  }));
}

const globalCatalogWithInvalidCountryCode: ProxyCatalogResponse = {
  view: "global",
  project_id: null,
  groups: [
    {
      import: {
        import_id: "imp-test",
        name: "global-jp",
        import_kind: "subscription",
        source_scope: { type: "global" },
        source_identity: {
          source_type: "url",
          source_value: "https://example.test/global-jp.yaml",
        },
        allocation_scope: { type: "global" },
        effective_project_ids: ["default"],
        proxy_count: 1,
        distinct_ip_count: 1,
        created_at: 1_713_308_400,
        updated_at: 1_713_309_000,
      },
      nodes: [
        {
          import_id: "imp-test",
          node_id: "node-invalid-country-code",
          proxy_name: "JP-Tokyo-Entry",
          proxy_type: "vmess",
          server: "tokyo-a.example.com:443",
          resolved_ips: ["203.0.113.10"],
          source_scope: { type: "global" },
          allocation_scope: { type: "global" },
          effective_project_ids: ["default"],
          primary_ip: "203.0.113.10",
          can_open_session: false,
          ip_metadata: [
            {
              node_id: "node-invalid-country-code",
              ip: "203.0.113.10",
              country_code: "global",
              country_name: "Japan",
              region_name: "Tokyo",
              city: "Chiyoda",
              geo_source: "geoip",
              probe_updated_at: 1_713_309_300,
              geo_updated_at: 1_713_309_200,
              last_probe_ok: true,
              last_latency_ms: 88,
              median_latency_ms: 92,
              last_probe_samples: [90, 88, 92, 95, 91],
              recent_probe_samples: createProbeSamples(
                "node-invalid-country-code",
                "203.0.113.10",
                [88, 91, 95, 92, 90],
              ),
              updated_at: 1_713_309_300,
            },
          ],
        },
      ],
    },
  ],
};

describe("ProxiesPage", () => {
  it("renders global catalog rows even when geo metadata contains an invalid country code", () => {
    const onRefreshImports = vi.fn();
    render(
      <I18nProvider initialLocale="en-US">
        <TooltipProvider>
          <ProxiesPage
            mode="global"
            projects={["default"]}
            currentUser={currentUser}
            accessDenied={false}
            authError={null}
            globalLoadResponse={null}
            globalLoadError={null}
            loadingGlobal={false}
            proxyImports={null}
            proxyImportsLoading={false}
            proxyImportsError={null}
            reallocatingImportId={null}
            deletingImportId={null}
            proxyCatalog={globalCatalogWithInvalidCountryCode}
            proxyCatalogLoading={false}
            proxyCatalogError={null}
            liveConnectionState="connected"
            liveNodeStates={{}}
            queueingOperation={false}
            onLoadGlobal={vi.fn()}
            onReassignImport={vi.fn()}
            onDeleteImport={vi.fn()}
            onRefreshImports={onRefreshImports}
            onRefreshNodes={vi.fn()}
            onProbeNodes={vi.fn()}
          />
        </TooltipProvider>
      </I18nProvider>,
    );

    expect(screen.getByText("Grouped proxy catalog")).toBeInTheDocument();
    expect(screen.getByText("JP-Tokyo-Entry")).toBeInTheDocument();
    expect(screen.getByText("Japan / Chiyoda")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Update$/i })).toBeInTheDocument();
  });

  it("refreshes a subscription import from a refreshable import group", async () => {
    const user = userEvent.setup();
    const onRefreshImports = vi.fn();
    render(
      <I18nProvider initialLocale="en-US">
        <TooltipProvider>
          <ProxiesPage
            mode="global"
            projects={["default"]}
            currentUser={currentUser}
            accessDenied={false}
            authError={null}
            globalLoadResponse={null}
            globalLoadError={null}
            loadingGlobal={false}
            proxyImports={null}
            proxyImportsLoading={false}
            proxyImportsError={null}
            reallocatingImportId={null}
            deletingImportId={null}
            proxyCatalog={globalCatalogWithInvalidCountryCode}
            proxyCatalogLoading={false}
            proxyCatalogError={null}
            liveConnectionState="connected"
            liveNodeStates={{}}
            queueingOperation={false}
            onLoadGlobal={vi.fn()}
            onReassignImport={vi.fn()}
            onDeleteImport={vi.fn()}
            onRefreshImports={onRefreshImports}
            onRefreshNodes={vi.fn()}
            onProbeNodes={vi.fn()}
          />
        </TooltipProvider>
      </I18nProvider>,
    );

    await user.click(screen.getByRole("button", { name: /^Update$/i }));

    expect(onRefreshImports).toHaveBeenCalledWith(["imp-test"]);
  });

  it("hides subscription refresh controls for non-admin users", () => {
    render(
      <I18nProvider initialLocale="en-US">
        <TooltipProvider>
          <ProxiesPage
            mode="global"
            projects={["default"]}
            currentUser={nonAdminUser}
            accessDenied={false}
            authError={null}
            globalLoadResponse={null}
            globalLoadError={null}
            loadingGlobal={false}
            proxyImports={null}
            proxyImportsLoading={false}
            proxyImportsError={null}
            reallocatingImportId={null}
            deletingImportId={null}
            proxyCatalog={globalCatalogWithInvalidCountryCode}
            proxyCatalogLoading={false}
            proxyCatalogError={null}
            liveConnectionState="connected"
            liveNodeStates={{}}
            queueingOperation={false}
            onLoadGlobal={vi.fn()}
            onReassignImport={vi.fn()}
            onDeleteImport={vi.fn()}
            onRefreshImports={vi.fn()}
            onRefreshNodes={vi.fn()}
            onProbeNodes={vi.fn()}
          />
        </TooltipProvider>
      </I18nProvider>,
    );

    expect(screen.queryByRole("button", { name: /^Update$/i })).not.toBeInTheDocument();
  });
});
