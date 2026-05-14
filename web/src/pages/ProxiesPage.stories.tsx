import type { Meta, StoryObj } from "@storybook/react-vite";
import { type ReactNode, useState } from "react";
import { toast } from "sonner";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { AppShell } from "@/components/AppShell";
import { Toaster } from "@/components/ui/sonner";
import type { ProxyNodeLiveState } from "@/hooks/use-proxy-operation-events";
import { GLOBAL_PROJECT_ID } from "@/lib/project-selection";
import type {
  CurrentUserState,
  ProjectProxySettings,
  ProxyCatalogResponse,
  ProxyNodeProbeSampleRecord,
} from "@/lib/types";
import {
  DeleteImportConfirmDialog,
  NodePinnedBatchDialog,
  NodePinnedSessionDialog,
  ProxiesPage,
  type ProxiesPageProps,
} from "@/pages/ProxiesPage";

const projects = ["default", "edge-jp", "lab-us"];

const currentUser: CurrentUserState = {
  status: "resolved",
  identity: {
    authenticated: true,
    principal_type: "human",
    subject: "admin@example.com",
    email: "admin@example.com",
    groups: ["admins", "ops"],
    is_admin: true,
  },
};

function createProbeSamples(
  nodeId: string,
  ip: string,
  samples: Array<number | null>,
  sampledAt = 1_713_309_380,
): ProxyNodeProbeSampleRecord[] {
  return samples.map((latencyMs, index) => ({
    node_id: nodeId,
    ip,
    target_url: "https://www.gstatic.com/generate_204",
    ok: latencyMs != null,
    latency_ms: latencyMs,
    sampled_at: sampledAt - index * 20,
  }));
}

const proxyImportsFixture = {
  items: [
    {
      import_id: "imp-M7n2Qa8Wx4Rp7Ts1",
      name: "global-jp",
      import_kind: "subscription" as const,
      source_scope: { type: "global" as const },
      source_identity: {
        source_type: "url",
        source_value: "https://example.com/global-jp.yaml",
      },
      allocation_scope: { type: "global" as const },
      effective_project_ids: ["default", "edge-jp", "lab-us"],
      proxy_count: 12,
      distinct_ip_count: 9,
      subscription_metadata: {
        source_title: "Tokyo Premium Feed",
        upload_bytes: 10 * 1024 ** 3,
        download_bytes: 20 * 1024 ** 3,
        used_bytes: 30 * 1024 ** 3,
        total_bytes: 100 * 1024 ** 3,
        remaining_bytes: 70 * 1024 ** 3,
        expire_at: 1_741_748_800,
      },
      created_at: 1_713_308_400,
      updated_at: 1_713_309_000,
    },
    {
      import_id: "imp-V5k3Ld9Hq2Cx8Zm4",
      name: "edge-manual",
      import_kind: "single_node" as const,
      source_scope: { type: "project" as const, project_id: "edge-jp" },
      source_identity: {
        source_type: "manual",
        source_value: "imp-V5k3Ld9Hq2Cx8Zm4",
      },
      allocation_scope: { type: "project" as const, project_id: "edge-jp" },
      effective_project_ids: ["edge-jp"],
      proxy_count: 4,
      distinct_ip_count: 3,
      subscription_metadata: null,
      created_at: 1_713_308_800,
      updated_at: 1_713_309_200,
    },
  ],
};

const globalCatalogFixture: ProxyCatalogResponse = {
  view: "global",
  project_id: null,
  groups: [
    {
      import: proxyImportsFixture.items[0],
      nodes: [
        {
          import_id: "imp-M7n2Qa8Wx4Rp7Ts1",
          node_id: "node-jp-tokyo-entry",
          proxy_name: "JP-Tokyo-Entry",
          proxy_type: "vmess",
          server: "tokyo-a.example.com:443",
          resolved_ips: ["203.0.113.10"],
          source_scope: { type: "global" },
          allocation_scope: { type: "global" },
          effective_project_ids: ["default", "edge-jp", "lab-us"],
          primary_ip: "203.0.113.10",
          can_open_session: false,
          ip_metadata: [
            {
              node_id: "node-jp-tokyo-entry",
              ip: "203.0.113.10",
              country_code: "JP",
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
              recent_probe_samples: createProbeSamples("node-jp-tokyo-entry", "203.0.113.10", [
                88,
                91,
                95,
                92,
                90,
                140,
                151,
                210,
                309,
                null,
              ]),
              updated_at: 1_713_309_300,
            },
          ],
        },
        {
          import_id: "imp-M7n2Qa8Wx4Rp7Ts1",
          node_id: "node-jp-osaka-edge",
          proxy_name: "JP-Osaka-Edge",
          proxy_type: "trojan",
          server: "osaka-b.example.com:443",
          resolved_ips: ["203.0.113.88"],
          source_scope: { type: "global" },
          allocation_scope: { type: "global" },
          effective_project_ids: ["default", "edge-jp"],
          primary_ip: "203.0.113.88",
          can_open_session: false,
          ip_metadata: [
            {
              node_id: "node-jp-osaka-edge",
              ip: "203.0.113.88",
              country_code: "JP",
              country_name: "Japan",
              region_name: "Osaka",
              city: "Osaka",
              geo_source: "geoip",
              probe_updated_at: 1_713_309_320,
              geo_updated_at: 1_713_309_200,
              last_probe_ok: false,
              last_latency_ms: null,
              median_latency_ms: null,
              last_probe_samples: [null, null, null, null, null],
              recent_probe_samples: createProbeSamples("node-jp-osaka-edge", "203.0.113.88", [
                null,
                null,
                null,
                318,
                null,
              ]),
              updated_at: 1_713_309_320,
            },
          ],
        },
      ],
    },
    {
      import: proxyImportsFixture.items[1],
      nodes: [
        {
          import_id: "imp-V5k3Ld9Hq2Cx8Zm4",
          node_id: "node-edge-manual-1",
          proxy_name: "Edge-Manual-1",
          proxy_type: "ss",
          server: "edge-jp.example.com:8443",
          resolved_ips: ["198.51.100.42"],
          source_scope: { type: "project", project_id: "edge-jp" },
          allocation_scope: { type: "project", project_id: "edge-jp" },
          effective_project_ids: ["edge-jp"],
          primary_ip: "198.51.100.42",
          can_open_session: false,
          ip_metadata: [],
        },
      ],
    },
  ],
};

const projectCatalogFixture: ProxyCatalogResponse = {
  view: "project",
  project_id: "edge-jp",
  groups: globalCatalogFixture.groups.map((group) => ({
    import: group.import,
    nodes: group.nodes.map((node) => ({
      ...node,
      can_open_session: true,
    })),
  })),
};

const globalCatalogMalformedGeoFixture: ProxyCatalogResponse = {
  ...globalCatalogFixture,
  groups: globalCatalogFixture.groups.map((group, groupIndex) => ({
    ...group,
    nodes: group.nodes.map((node, nodeIndex) => ({
      ...node,
      ip_metadata:
        groupIndex === 0 && nodeIndex === 0
          ? node.ip_metadata.map((metadata, metadataIndex) =>
              metadataIndex === 0 ? { ...metadata, country_code: "global" } : metadata,
            )
          : node.ip_metadata.map((metadata) => ({ ...metadata })),
    })),
  })),
};

const liveNodeStates: Record<string, ProxyNodeLiveState> = {
  "node-jp-osaka-edge": {
    kind: "proxy_latency_probe",
    runId: "run-probe-001",
    nodeId: "node-jp-osaka-edge",
    samplesTotal: 5,
    latestRound: 3,
    latestSampleMs: null,
    at: 1_713_309_350,
    message: "probe round 3 timeout",
  },
};

const projectSettingsFixture: ProjectProxySettings = {
  project_id: "edge-jp",
  use_global_proxies: true,
};

const meta = {
  title: "Pages/ProxiesPage",
  component: ProxiesPage,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          "Unified proxy workspace that follows the current project selector. Pick Global to manage the shared pool and allocations, or pick a project to manage local imports, grouped nodes, and node-pinned session creation.",
      },
    },
  },
} satisfies Meta<typeof ProxiesPage>;

export default meta;
type Story = StoryObj<typeof meta>;

function createProjectCatalogFixture(): ProxyCatalogResponse {
  return {
    view: projectCatalogFixture.view,
    project_id: projectCatalogFixture.project_id,
    groups: projectCatalogFixture.groups.map((group) => ({
      import: {
        ...group.import,
        source_scope: { ...group.import.source_scope },
        source_identity: { ...group.import.source_identity },
        allocation_scope: { ...group.import.allocation_scope },
        effective_project_ids: [...group.import.effective_project_ids],
      },
      nodes: group.nodes.map((node) => ({
        ...node,
        source_scope: { ...node.source_scope },
        allocation_scope: { ...node.allocation_scope },
        effective_project_ids: [...node.effective_project_ids],
        resolved_ips: [...node.resolved_ips],
        ip_metadata: node.ip_metadata.map((metadata) => ({
          ...metadata,
          last_probe_samples: [...metadata.last_probe_samples],
          recent_probe_samples: metadata.recent_probe_samples.map((sample) => ({ ...sample })),
        })),
      })),
    })),
  };
}

function createLiveNodeStates(): Record<string, ProxyNodeLiveState> {
  return Object.fromEntries(
    Object.entries(liveNodeStates).map(([nodeId, state]) => [nodeId, { ...state }]),
  );
}

function sleep(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function InteractiveProjectStory(args: Extract<ProxiesPageProps, { mode: "project" }>) {
  const [proxyCatalog, setProxyCatalog] = useState<ProxyCatalogResponse>(
    createProjectCatalogFixture,
  );
  const [proxySettings, setProxySettings] = useState<ProjectProxySettings>(projectSettingsFixture);
  const [liveState, setLiveState] =
    useState<Record<string, ProxyNodeLiveState>>(createLiveNodeStates);
  const [queueingOperation, setQueueingOperation] = useState(false);
  const [openingSessionNodeId, setOpeningSessionNodeId] = useState<string | null>(null);
  const [openingBatch, setOpeningBatch] = useState(false);
  const [deletingImportId, setDeletingImportId] = useState<string | null>(null);
  const [syncingImportIds, setSyncingImportIds] = useState<string[]>([]);
  const [feedback, setFeedback] = useState<{
    title: string;
    description: string;
    tone: "success" | "warning";
  } | null>({
    title: "Interactive mock controls",
    description:
      "Use this story like a mini control room: local imports can be deleted, probe refreshes node state, and create-session buttons surface mock operator feedback inline.",
    tone: "warning",
  });

  const patchNodeMetadata = (nodeIds: string[], mutator: (nodeId: string) => number | null) => {
    setProxyCatalog((current) => ({
      ...current,
      groups: current.groups.map((group) => ({
        ...group,
        nodes: group.nodes.map((node) => {
          if (!nodeIds.includes(node.node_id)) {
            return node;
          }
          const latency = mutator(node.node_id);
          const updatedAt = 1_713_309_900;
          const samples =
            latency == null
              ? [null, null, null, null, null]
              : [latency - 5, latency, latency + 3, latency + 1, latency - 2];
          const ip = node.primary_ip ?? node.resolved_ips[0] ?? "198.51.100.42";
          const recentProbeSamples = createProbeSamples(node.node_id, ip, samples, updatedAt);

          const metadata = node.ip_metadata[0]
            ? {
                ...node.ip_metadata[0],
                updated_at: updatedAt,
                probe_updated_at: updatedAt,
                median_latency_ms: latency,
                last_latency_ms: latency,
                last_probe_ok: latency != null,
                last_probe_samples: samples,
                recent_probe_samples: recentProbeSamples,
              }
            : {
                node_id: node.node_id,
                ip,
                country_code: "JP",
                country_name: "Japan",
                region_name: node.node_id === "node-edge-manual-1" ? "Tokyo" : "Osaka",
                city: node.node_id === "node-edge-manual-1" ? "Shibuya" : "Osaka",
                geo_source: "storybook",
                probe_updated_at: updatedAt,
                geo_updated_at: updatedAt,
                last_probe_ok: latency != null,
                last_latency_ms: latency,
                median_latency_ms: latency,
                last_probe_samples: samples,
                recent_probe_samples: recentProbeSamples,
                updated_at: updatedAt,
              };

          return {
            ...node,
            ip_metadata: [metadata],
          };
        }),
      })),
    }));
  };

  return (
    <AppShell
      projectId="edge-jp"
      projects={projects}
      projectsLoading={false}
      projectsCreating={false}
      projectsError={null}
      healthStatus="ok"
      currentUser={currentUser}
      onProjectIdChange={() => undefined}
      onCreateProject={async (value: string) => value}
      onRetryProjects={() => undefined}
    >
      <div className="space-y-5">
        <Toaster richColors position="top-right" />
        {feedback ? (
          <ActionResponsePanel
            title={feedback.title}
            description={feedback.description}
            tone={feedback.tone}
          />
        ) : null}
        <ProxiesPage
          {...args}
          proxyCatalog={proxyCatalog}
          proxySettings={proxySettings}
          liveNodeStates={liveState}
          queueingOperation={queueingOperation}
          deletingImportId={deletingImportId}
          syncingImportIds={syncingImportIds}
          openingSessionNodeId={openingSessionNodeId}
          openingBatch={openingBatch}
          suggestedPort={10080}
          onToggleUseGlobalProxies={async (nextValue) => {
            setProxySettings((current) => ({ ...current, use_global_proxies: nextValue }));
            toast.success(nextValue ? "Global pool enabled" : "Global pool disabled", {
              description: nextValue
                ? "edge-jp now composes inherited global nodes together with its local imports."
                : "edge-jp now restricts the effective pool to project-local imports only.",
            });
            setFeedback({
              title: nextValue ? "Global pool enabled" : "Global pool disabled",
              description: nextValue
                ? "edge-jp now composes inherited global nodes together with its local imports."
                : "edge-jp now restricts the effective pool to project-local imports only.",
              tone: "success",
            });
          }}
          onRefreshNodes={async (nodeIds) => {
            setQueueingOperation(true);
            setLiveState((current) =>
              Object.fromEntries(
                Object.entries(current).filter(([nodeId]) => !nodeIds.includes(nodeId)),
              ),
            );
            setFeedback({
              title: "Metadata refresh queued",
              description: `Refreshing metadata for ${nodeIds.length} selected node(s).`,
              tone: "warning",
            });
            toast.loading(`Refreshing ${nodeIds.length} node(s)…`, {
              description: "Mock metadata refresh is running inside this story.",
              id: "project-refresh",
            });
            await sleep(220);
            patchNodeMetadata(nodeIds, (nodeId) =>
              nodeId === "node-edge-manual-1" ? 118 : nodeId === "node-jp-tokyo-entry" ? 89 : null,
            );
            setQueueingOperation(false);
            toast.success("Metadata refresh applied", {
              description: "Selected nodes now show refreshed metadata.",
              id: "project-refresh",
            });
            setFeedback({
              title: "Metadata refresh applied",
              description:
                "Selected nodes now show refreshed geo metadata and the latest mock probe median.",
              tone: "success",
            });
          }}
          onProbeNodes={async (nodeIds) => {
            setQueueingOperation(true);
            toast.loading(`Probing ${nodeIds.length} node(s)…`, {
              description: "Watch the status column update while mock rounds progress.",
              id: "project-probe",
            });
            setLiveState(
              Object.fromEntries(
                nodeIds.map((nodeId) => [
                  nodeId,
                  {
                    kind: "proxy_latency_probe",
                    runId: `run-${nodeId}`,
                    nodeId,
                    samplesTotal: 5,
                    latestRound: 1,
                    latestSampleMs: nodeId === "node-jp-osaka-edge" ? null : 101,
                    at: 1_713_309_600,
                    message: "probe round 1 sample",
                  },
                ]),
              ),
            );
            setFeedback({
              title: "Latency probe running",
              description:
                "The node status column updates in place while the mock breadth-first rounds progress.",
              tone: "warning",
            });
            await sleep(220);
            setLiveState(
              Object.fromEntries(
                nodeIds.map((nodeId) => [
                  nodeId,
                  {
                    kind: "proxy_latency_probe",
                    runId: `run-${nodeId}`,
                    nodeId,
                    samplesTotal: 5,
                    latestRound: 5,
                    latestSampleMs: nodeId === "node-jp-osaka-edge" ? null : 97,
                    at: 1_713_309_800,
                    message:
                      nodeId === "node-jp-osaka-edge"
                        ? "probe round 5 timeout"
                        : "probe round 5 ok",
                  },
                ]),
              ),
            );
            await sleep(220);
            patchNodeMetadata(nodeIds, (nodeId) =>
              nodeId === "node-jp-osaka-edge" ? null : nodeId === "node-edge-manual-1" ? 114 : 97,
            );
            setLiveState({});
            setQueueingOperation(false);
            toast.success("Latency probe finished", {
              description:
                "Successful nodes keep the final median; timeout-only nodes remain failed.",
              id: "project-probe",
            });
            setFeedback({
              title: "Latency probe finished",
              description:
                "Mock rounds completed. Successful nodes keep the final median while timeout-only nodes stay failed.",
              tone: "success",
            });
          }}
          onDeleteImport={async (importId) => {
            setDeletingImportId(importId);
            await sleep(220);
            setProxyCatalog((current) => ({
              ...current,
              groups: current.groups.filter((group) => group.import.import_id !== importId),
            }));
            setDeletingImportId(null);
            toast.success("Local import removed", {
              description: `${importId} was removed from this project-only story state.`,
            });
            setFeedback({
              title: "Local import removed",
              description:
                "The local node-group import was deleted from this project view. Inherited global imports remain protected here.",
              tone: "success",
            });
          }}
          onSyncImports={async (importIds) => {
            setSyncingImportIds(importIds);
            toast.loading("Updating subscription…", {
              description: `Queued mock subscription update for ${importIds.join(", ")}.`,
              id: "sync-imports",
            });
            await sleep(220);
            setProxyCatalog((current) => ({
              ...current,
              groups: current.groups.map((group) =>
                importIds.includes(group.import.import_id)
                  ? {
                      ...group,
                      import: {
                        ...group.import,
                        proxy_count: group.import.proxy_count + 1,
                        updated_at: 1_713_310_000,
                      },
                    }
                  : group,
              ),
            }));
            setSyncingImportIds([]);
            toast.success("Subscription update queued", {
              description: "The grouped import now reflects a refreshed mock timestamp.",
              id: "sync-imports",
            });
            setFeedback({
              title: "Subscription update queued",
              description:
                "The selected source-based subscription import refreshed its mock catalog timestamp. Manual node groups do not expose this action.",
              tone: "success",
            });
          }}
          onOpenSessionByNode={async ({ node_id: nodeId, desired_port: desiredPort }) => {
            setOpeningSessionNodeId(nodeId);
            toast.loading("Creating node-pinned session…", {
              description:
                desiredPort != null
                  ? `${nodeId} is opening on its primary resolved IP via port ${desiredPort}.`
                  : `${nodeId} is opening on its primary resolved IP.`,
              id: "open-session",
            });
            await sleep(220);
            setOpeningSessionNodeId(null);
            toast.success("Session created", {
              description:
                desiredPort != null
                  ? `${nodeId} now has a mock live listener on port ${desiredPort}.`
                  : `${nodeId} now has a mock live listener bound to its primary IP.`,
              id: "open-session",
            });
            setFeedback({
              title: "Node-pinned session created",
              description:
                desiredPort != null
                  ? `Mock listener opened for ${nodeId} on requested port ${desiredPort}.`
                  : `Mock listener opened for ${nodeId} using the primary resolved IP.`,
              tone: "success",
            });
          }}
          onOpenBatchByNode={async (payload) => {
            const requests =
              payload.requests ?? (payload.node_ids ?? []).map((nodeId) => ({ node_id: nodeId }));
            setOpeningBatch(true);
            toast.loading("Creating batch sessions…", {
              description: `Opening ${requests.length} node-pinned listener(s).`,
              id: "open-batch",
            });
            await sleep(220);
            setOpeningBatch(false);
            toast.success("Batch sessions created", {
              description: `Opened ${requests.length} mock listener(s), one per selected node.`,
              id: "open-batch",
            });
            setFeedback({
              title: "Batch sessions created",
              description: `Opened ${requests.length} mock listener(s), one per selected node.`,
              tone: "success",
            });
          }}
          onLoadProject={async () => {
            toast.success("Local import form submitted", {
              description:
                "The form stays non-destructive in Storybook, but the grouped-node controls below are fully interactive.",
            });
            setFeedback({
              title: "Local import form submitted",
              description:
                "This story keeps the import form non-destructive, but the surrounding grouped-node controls remain fully interactive.",
              tone: "success",
            });
          }}
        />
      </div>
    </AppShell>
  );
}

function renderInShell(storyArgs: Story["args"]) {
  const projectId = storyArgs?.mode === "project" ? storyArgs.projectId : GLOBAL_PROJECT_ID;
  return (
    <AppShell
      projectId={projectId}
      projects={projects}
      projectsLoading={false}
      projectsCreating={false}
      projectsError={null}
      healthStatus="ok"
      currentUser={currentUser}
      onProjectIdChange={() => undefined}
      onCreateProject={async (value: string) => value}
      onRetryProjects={() => undefined}
    >
      <ProxiesPage {...storyArgs} />
    </AppShell>
  );
}

function renderProjectSurface(
  storyArgs: Extract<ProxiesPageProps, { mode: "project" }>,
  overlay?: ReactNode,
) {
  return (
    <AppShell
      projectId={storyArgs.projectId}
      projects={projects}
      projectsLoading={false}
      projectsCreating={false}
      projectsError={null}
      healthStatus="ok"
      currentUser={currentUser}
      onProjectIdChange={() => undefined}
      onCreateProject={async (value: string) => value}
      onRetryProjects={() => undefined}
    >
      <div className="contents">
        <ProxiesPage {...storyArgs} />
        {overlay}
      </div>
    </AppShell>
  );
}

export const GlobalProject: Story = {
  args: {
    mode: "global",
    projects,
    currentUser,
    accessDenied: false,
    authError: null,
    globalLoadResponse: {
      loaded_proxies: 12,
      distinct_ips: 9,
      resolved_name: "global-jp",
      resolved_name_source: "parsed_source",
      subscription_metadata: proxyImportsFixture.items[0]?.subscription_metadata,
      warnings: [],
    },
    globalLoadError: null,
    loadingGlobal: false,
    proxyImports: proxyImportsFixture,
    proxyImportsLoading: false,
    proxyImportsError: null,
    reallocatingImportId: null,
    deletingImportId: null,
    proxyCatalog: globalCatalogFixture,
    proxyCatalogLoading: false,
    proxyCatalogError: null,
    systemSettings: {
      proxy_probe_interval_sec: 3600,
      updated_at: 1_713_309_300,
    },
    systemSettingsLoading: false,
    systemSettingsError: null,
    updatingSystemSettings: false,
    liveConnectionState: "connected",
    liveNodeStates,
    queueingOperation: false,
    onLoadGlobal: fn(),
    onUpdateSystemSettings: fn(),
    onReassignImport: fn(),
    onDeleteImport: fn(),
    onRefreshNodes: fn(),
    onProbeNodes: fn(),
    onSyncImports: fn(),
  },
  render: renderInShell,
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(/Grouped proxy catalog/i)).toBeVisible();
    await expect(await canvas.findByText(/^global-jp$/i)).toBeVisible();
    await expect(
      (await canvas.findAllByText(/Source title: Tokyo Premium Feed/i)).length,
    ).toBeGreaterThan(0);
    await expect((await canvas.findAllByText(/Remaining/i)).length).toBeGreaterThan(0);
    await expect(await canvas.findByText(/JP-Tokyo-Entry/i)).toBeVisible();
    await expect(await canvas.findByText(/Automatic latency probe/i)).toBeVisible();
    await expect(await canvas.findByText(/88 ms/i)).toBeVisible();
    await userEvent.hover(await canvas.findByText(/88 ms/i));
    const dialog = within(canvasElement.ownerDocument.body);
    await expect((await dialog.findAllByText(/309 ms/i)).length).toBeGreaterThan(0);
    await expect(await canvas.findByText(/Probe selected/i)).toBeVisible();
  },
};

export const ConcreteProject: Story = {
  args: {
    mode: "project",
    projectId: "edge-jp",
    currentUser,
    projectLoadResponse: {
      loaded_proxies: 4,
      distinct_ips: 3,
      resolved_name: "edge-feed",
      resolved_name_source: "parsed_source",
      subscription_metadata: {
        source_title: "Edge JP Feed",
        total_bytes: 60 * 1024 ** 3,
        remaining_bytes: 42 * 1024 ** 3,
        used_bytes: 18 * 1024 ** 3,
        expire_at: 1_741_760_000,
      },
      warnings: ["filtered informational subscription entry `剩余流量 42GB`"],
    },
    projectLoadError: null,
    loadingProject: false,
    proxySettings: {
      project_id: "edge-jp",
      use_global_proxies: true,
    },
    proxySettingsLoading: false,
    proxySettingsError: null,
    updatingSettings: false,
    showProxyPolicy: true,
    proxyCatalog: projectCatalogFixture,
    proxyCatalogLoading: false,
    proxyCatalogError: null,
    liveConnectionState: "connected",
    liveNodeStates,
    queueingOperation: false,
    suggestedPort: 10080,
    openingSessionNodeId: null,
    openingBatch: false,
    onLoadProject: fn(),
    onToggleUseGlobalProxies: fn(),
    onRefreshNodes: fn(),
    onProbeNodes: fn(),
    onSyncImports: fn(),
    onDeleteImport: fn(),
    onOpenSessionByNode: fn(),
    onOpenBatchByNode: fn(),
    deletingImportId: null,
  },
  render: (args) => (
    <InteractiveProjectStory {...(args as Extract<ProxiesPageProps, { mode: "project" }>)} />
  ),
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    const dialog = within(canvasElement.ownerDocument.body);
    await expect(await canvas.findByText(/Current project grouped nodes/i)).toBeVisible();
    await expect(await canvas.findByRole("button", { name: /^Create sessions$/i })).toBeVisible();
    await expect((await canvas.findAllByRole("button", { name: /^Delete$/i })).length).toBe(1);
    await userEvent.click(await canvas.findByRole("button", { name: /^Delete$/i }));
    await expect(await dialog.findByText(/Confirm deletion/i)).toBeVisible();
    await userEvent.click(await dialog.findByRole("button", { name: /^Cancel$/i }));
    await waitFor(() => {
      expect(dialog.queryByRole("dialog", { name: /Confirm deletion/i })).not.toBeInTheDocument();
    });
    await userEvent.click(await canvas.findByRole("button", { name: /^Delete$/i }));
    await userEvent.click(await dialog.findByRole("button", { name: /^Delete$/i }));
    await waitFor(() => {
      expect(canvas.queryByText(/^edge-manual$/i)).not.toBeInTheDocument();
    });
    const createSessionButtons = await canvas.findAllByRole("button", {
      name: /^Create session$/i,
    });
    await expect(createSessionButtons.length).toBeGreaterThan(0);
    const firstCreateSessionButton = createSessionButtons[0];
    if (!firstCreateSessionButton) {
      throw new Error("Expected at least one Create session button");
    }
    await userEvent.click(firstCreateSessionButton);
    const desiredPortInput = await dialog.findByLabelText(/Desired port \(optional\)/i);
    await userEvent.clear(desiredPortInput);
    await userEvent.type(desiredPortInput, "10088");
    await userEvent.click(await dialog.findByRole("button", { name: /^Create session$/i }));
    await expect(await canvas.findByText(/Node-pinned session created/i)).toBeVisible();
  },
};

export const ZhCN: Story = {
  ...GlobalProject,
  globals: {
    locale: "zh-CN",
  },
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(/分组代理目录/i)).toBeVisible();
    await expect(await canvas.findByText(/刷新所选/i)).toBeVisible();
    await expect(await canvas.findByText(/^global-jp$/i)).toBeVisible();
  },
};

export const GlobalMalformedGeoMetadata: Story = {
  ...GlobalProject,
  name: "Global Malformed Geo Metadata",
  args: {
    ...GlobalProject.args,
    proxyCatalog: globalCatalogMalformedGeoFixture,
  },
  globals: {
    locale: "zh-CN",
  },
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(/分组代理目录/i)).toBeVisible();
    await expect(await canvas.findByText(/日本 \/ Chiyoda/i)).toBeVisible();
    await expect(canvas.queryByText(/Unexpected Application Error/i)).not.toBeInTheDocument();
  },
};

export const ProjectCatalog: Story = {
  args: { ...ConcreteProject.args },
  name: "Project Catalog",
  render: (args) => renderInShell(args),
};

export const ProjectBatchActions: Story = {
  ...ConcreteProject,
  render: (args) => (
    <InteractiveProjectStory {...(args as Extract<ProxiesPageProps, { mode: "project" }>)} />
  ),
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    const dialog = within(canvasElement.ownerDocument.body);
    const groupCheckbox = await canvas.findByLabelText(/Select import group global-jp/i);
    await userEvent.click(groupCheckbox);
    const selectedBadges = await canvas.findAllByText(/Selected 2 nodes/i);
    await expect(selectedBadges.length).toBeGreaterThan(0);
    await expect(await canvas.findByRole("button", { name: /Create sessions/i })).toBeEnabled();
    await userEvent.click(await canvas.findByRole("button", { name: /Create sessions/i }));
    const desiredPortInputs = await dialog.findAllByLabelText(/Desired port \(optional\)/i);
    const firstDesiredPortInput = desiredPortInputs[0];
    const secondDesiredPortInput = desiredPortInputs[1];
    if (!firstDesiredPortInput || !secondDesiredPortInput) {
      throw new Error("Expected desired port inputs for the selected batch nodes");
    }
    await userEvent.type(firstDesiredPortInput, "10080");
    await userEvent.type(secondDesiredPortInput, "10081");
    await userEvent.click(await dialog.findByRole("button", { name: /^Create sessions$/i }));
    const batchSuccessTitles = await canvas.findAllByText(/^Batch sessions created$/i);
    await expect(batchSuccessTitles.length).toBeGreaterThan(0);
  },
};

export const ProjectSelectionFlow: Story = {
  ...ConcreteProject,
  render: (args) => (
    <InteractiveProjectStory {...(args as Extract<ProxiesPageProps, { mode: "project" }>)} />
  ),
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    const groupCheckbox = await canvas.findByLabelText(/Select import group global-jp/i);
    const firstCheckbox = await canvas.findByLabelText(/Select node JP-Tokyo-Entry/i);
    const secondCheckbox = await canvas.findByLabelText(/Select node JP-Osaka-Edge/i);
    const thirdCheckbox = await canvas.findByLabelText(/Select node Edge-Manual-1/i);
    firstCheckbox.focus();
    await userEvent.keyboard("[Space]");
    await expect(groupCheckbox).toHaveAttribute("data-state", "indeterminate");

    firstCheckbox.focus();
    await userEvent.keyboard("[Space]");
    firstCheckbox.focus();
    await userEvent.keyboard("[Space]");
    secondCheckbox.focus();
    await userEvent.keyboard("[Space]");
    await expect((await canvas.findAllByText(/Selected 2 nodes/i)).length).toBeGreaterThan(0);

    thirdCheckbox.focus();
    await userEvent.keyboard("[Space]");
    await expect((await canvas.findAllByText(/Selected 3 nodes/i)).length).toBeGreaterThan(0);
    await expect(await canvas.findByRole("button", { name: /Create sessions/i })).toBeEnabled();
  },
};

export const ProjectCreateSessionDialog: Story = {
  args: { ...ConcreteProject.args },
  name: "Project Create Session Dialog",
  render: (args) =>
    renderProjectSurface(
      args as Extract<ProxiesPageProps, { mode: "project" }>,
      <NodePinnedSessionDialog
        open
        node={projectCatalogFixture.groups[0]?.nodes[0] ?? null}
        suggestedPort={10080}
        isPending={false}
        onOpenChange={() => undefined}
        onSubmit={async () => undefined}
      />,
    ),
};

export const ProjectBatchCreateDialog: Story = {
  args: { ...ConcreteProject.args },
  name: "Project Batch Create Dialog",
  render: (args) =>
    renderProjectSurface(
      args as Extract<ProxiesPageProps, { mode: "project" }>,
      <NodePinnedBatchDialog
        open
        nodes={projectCatalogFixture.groups[0]?.nodes.slice(0, 2) ?? []}
        suggestedPort={10080}
        isPending={false}
        onOpenChange={() => undefined}
        onSubmit={async () => undefined}
      />,
    ),
};

export const ProjectDeleteConfirmDialog: Story = {
  args: { ...ConcreteProject.args },
  name: "Project Delete Confirm Dialog",
  render: (args) =>
    renderProjectSurface(
      args as Extract<ProxiesPageProps, { mode: "project" }>,
      <DeleteImportConfirmDialog
        open
        item={proxyImportsFixture.items[1]}
        isPending={false}
        onOpenChange={() => undefined}
        onConfirm={async () => undefined}
      />,
    ),
};

export const AccessDenied: Story = {
  args: {
    mode: "global",
    projects,
    currentUser,
    accessDenied: true,
    authError: null,
    globalLoadResponse: null,
    globalLoadError: null,
    loadingGlobal: false,
    proxyImports: null,
    proxyImportsLoading: false,
    proxyImportsError: null,
    proxyCatalog: null,
    proxyCatalogLoading: false,
    proxyCatalogError: null,
    liveConnectionState: "idle",
    liveNodeStates: {},
    queueingOperation: false,
    onLoadGlobal: fn(),
    onReassignImport: fn(),
    onDeleteImport: fn(),
    onRefreshNodes: fn(),
    onProbeNodes: fn(),
    onSyncImports: fn(),
  },
  render: renderInShell,
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(/admin access required/i)).toBeVisible();
  },
};
