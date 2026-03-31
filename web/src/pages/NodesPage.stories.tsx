import type { Meta, StoryObj } from "@storybook/react-vite";
import { type ComponentProps, useEffect, useMemo, useState } from "react";
import { expect, fn, userEvent, within } from "storybook/test";

import { AppShell } from "@/components/AppShell";
import { defaultNodeFilterState, type NodeFilterState, splitFilterInput } from "@/lib/nodes-view";
import type { NodeExportFormat, NodeListItem, NodeListResponse, NodeViewMode } from "@/lib/types";
import { nodesFixture } from "@/mocks/fixtures";
import { NodesPage } from "@/pages/NodesPage";

const storyDefaultFilterState: NodeFilterState = {
  ...defaultNodeFilterState,
  pageSize: 3,
};

const interactiveNodeItems: NodeListItem[] = [
  nodesFixture.items[0],
  nodesFixture.items[1],
  {
    node_id: "KR-Seoul-Metro",
    proxy_name: "KR-Seoul-Metro",
    proxy_type: "vless",
    server: "seoul-metro.example.com",
    preferred_ip: "203.0.113.120",
    ipv4: "203.0.113.120",
    ipv6: "2001:db8::120",
    country_code: "KR",
    country_name: "South Korea",
    region_name: "Seoul",
    city: "Jung-gu",
    probe_status: "reachable",
    best_latency_ms: 74,
    last_used_at: 1_741_748_520,
    session_count: 3,
    subscription_type: "url",
    subscription_value: "https://mirror.example.com/edge.yaml",
  },
  {
    node_id: "SG-Singapore-Core",
    proxy_name: "SG-Singapore-Core",
    proxy_type: "hysteria2",
    server: "sg-core.example.com",
    preferred_ip: "203.0.113.150",
    ipv4: "203.0.113.150",
    ipv6: "2001:db8::150",
    country_code: "SG",
    country_name: "Singapore",
    region_name: "Singapore",
    city: "Downtown Core",
    probe_status: "reachable",
    best_latency_ms: 61,
    last_used_at: 1_741_748_640,
    session_count: 5,
    subscription_type: "url",
    subscription_value: "https://mirror.example.com/edge.yaml",
  },
  {
    node_id: "UK-London-Relay",
    proxy_name: "UK-London-Relay",
    proxy_type: "vmess",
    server: "london-relay.example.com",
    preferred_ip: "198.51.100.120",
    ipv4: "198.51.100.120",
    ipv6: null,
    country_code: "GB",
    country_name: "United Kingdom",
    region_name: "England",
    city: "London",
    probe_status: "reachable",
    best_latency_ms: 128,
    last_used_at: 1_741_748_760,
    session_count: 1,
    subscription_type: "file",
    subscription_value: "/etc/proxy-broker/subscriptions/lab.yaml",
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
    session_count: 1,
    subscription_type: "url",
    subscription_value: "https://example.com/subscription.yaml",
  },
].filter((item): item is NodeListItem => Boolean(item));

const interactiveNodesFixture: NodeListResponse = {
  total: interactiveNodeItems.length,
  page: 1,
  page_size: storyDefaultFilterState.pageSize,
  items: interactiveNodeItems,
};

function InteractiveNodesPageStory(args: ComponentProps<typeof NodesPage>) {
  const initialFilterState = useMemo(
    () => ({
      ...storyDefaultFilterState,
      ...args.filterState,
      pageSize: args.filterState.pageSize ?? storyDefaultFilterState.pageSize,
    }),
    [args.filterState],
  );
  const sourceItems = args.data?.items ?? [];
  const [filterState, setFilterState] = useState(initialFilterState);
  const [viewMode, setViewMode] = useState<NodeViewMode>(args.viewMode);
  const [bulkScope, setBulkScope] = useState<"selected" | "all_filtered">(args.bulkScope);
  const [selectedIds, setSelectedIds] = useState(args.selectedIds);
  const [isExporting, setIsExporting] = useState(args.isExporting ?? false);
  const [isOpening, setIsOpening] = useState(args.isOpening ?? false);

  useEffect(() => {
    setFilterState(initialFilterState);
  }, [initialFilterState]);

  useEffect(() => {
    setViewMode(args.viewMode);
  }, [args.viewMode]);

  useEffect(() => {
    setBulkScope(args.bulkScope);
  }, [args.bulkScope]);

  useEffect(() => {
    setSelectedIds(args.selectedIds);
  }, [args.selectedIds]);

  useEffect(() => {
    setIsExporting(args.isExporting ?? false);
  }, [args.isExporting]);

  useEffect(() => {
    setIsOpening(args.isOpening ?? false);
  }, [args.isOpening]);

  const data = useMemo(() => {
    if (!args.data) {
      return args.data;
    }

    return buildInteractiveResponse(sourceItems, filterState);
  }, [args.data, filterState, sourceItems]);

  const handleFilterChange = (patch: Partial<NodeFilterState>) => {
    setFilterState((current) => ({ ...current, ...patch }));
    args.onFilterChange(patch);
  };

  const handleResetFilters = () => {
    setFilterState(initialFilterState);
    args.onResetFilters();
  };

  const handleToggleSelect = (nodeId: string, checked: boolean) => {
    setSelectedIds((current) => {
      const next = checked
        ? Array.from(new Set([...current, nodeId]))
        : current.filter((id) => id !== nodeId);
      return next;
    });
    args.onToggleSelect(nodeId, checked);
  };

  const handleToggleSelectAll = (checked: boolean) => {
    setSelectedIds((current) => {
      const pageIds = data?.items.map((item) => item.node_id) ?? [];
      if (checked) {
        return Array.from(new Set([...current, ...pageIds]));
      }
      return current.filter((id) => !pageIds.includes(id));
    });
    args.onToggleSelectAll(checked);
  };

  const handleExport = async (format: NodeExportFormat) => {
    setIsExporting(true);
    try {
      await Promise.resolve(args.onExport(format));
    } finally {
      window.setTimeout(() => setIsExporting(false), 180);
    }
  };

  const handleOpenSessions = async () => {
    setIsOpening(true);
    try {
      await Promise.resolve(args.onOpenSessions());
    } finally {
      window.setTimeout(() => setIsOpening(false), 180);
    }
  };

  return (
    <NodesPage
      {...args}
      filterState={filterState}
      viewMode={viewMode}
      bulkScope={bulkScope}
      data={data}
      selectedIds={selectedIds}
      isExporting={isExporting}
      isOpening={isOpening}
      onFilterChange={handleFilterChange}
      onResetFilters={handleResetFilters}
      onViewModeChange={(value) => {
        setViewMode(value);
        args.onViewModeChange(value);
      }}
      onBulkScopeChange={(value) => {
        setBulkScope(value);
        args.onBulkScopeChange(value);
      }}
      onToggleSelect={handleToggleSelect}
      onToggleSelectAll={handleToggleSelectAll}
      onClearSelection={() => {
        setSelectedIds([]);
        args.onClearSelection();
      }}
      onExport={handleExport}
      onOpenSessions={handleOpenSessions}
    />
  );
}

const meta = {
  title: "Pages/NodesPage",
  component: NodesPage,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    initialEntries: ["/nodes"],
    docs: {
      description: {
        component:
          "Nodes route inside the real app shell, with server-driven filtering plus local regrouping and bulk actions.",
      },
    },
  },
  render: (args) => (
    <AppShell
      profileId="default"
      profiles={["default", "edge-jp", "lab-us"]}
      profilesLoading={false}
      profilesCreating={false}
      profilesError={null}
      healthStatus="ok"
      currentUser={{
        status: "resolved",
        identity: {
          authenticated: true,
          principal_type: "human",
          subject: "admin@example.com",
          email: "admin@example.com",
          groups: ["admins", "ops"],
          is_admin: true,
        },
      }}
      onProfileIdChange={() => undefined}
      onCreateProfile={async (value: string) => value}
      onRetryProfiles={() => undefined}
    >
      <InteractiveNodesPageStory {...args} />
    </AppShell>
  ),
  args: {
    filterState: storyDefaultFilterState,
    viewMode: "flat",
    bulkScope: "selected",
    data: interactiveNodesFixture,
    isLoading: false,
    isFetching: false,
    isExporting: false,
    isOpening: false,
    error: null,
    selectedIds: [],
    onFilterChange: fn(),
    onResetFilters: fn(),
    onViewModeChange: fn(),
    onBulkScopeChange: fn(),
    onToggleSelect: fn(),
    onToggleSelectAll: fn(),
    onClearSelection: fn(),
    onExport: fn(),
    onOpenSessions: fn(),
  },
} satisfies Meta<typeof NodesPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const overlay = within(canvasElement.ownerDocument.body);
    await userEvent.click(canvas.getByRole("tab", { name: /By region/i }));
    await expect(canvas.getByText("Japan / Tokyo / Chiyoda")).toBeInTheDocument();
    await userEvent.click(canvas.getAllByRole("checkbox")[1]);
    await expect(canvas.getByText("1 selected")).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: /^Export$/i }));
    await expect(
      overlay.getByRole("button", { name: /Node links \(.txt, one per line\)/i }),
    ).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: /^Next$/i }));
    await expect(canvas.getByText("SG-Singapore-Core")).toBeInTheDocument();
  },
};

export const ZhCN: Story = {
  globals: {
    locale: "zh-CN",
  },
};

export const Loading: Story = {
  args: {
    data: null,
    isLoading: true,
  },
};

export const EmptyState: Story = {
  args: {
    data: {
      total: 0,
      page: 1,
      page_size: storyDefaultFilterState.pageSize,
      items: [],
    },
  },
};

export const GroupedBySubscription: Story = {
  args: {
    viewMode: "group_by_subscription",
  },
};

export const BatchSelected: Story = {
  args: {
    selectedIds: ["JP-Tokyo-Entry", "US-SanJose-Fallback"],
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("2 selected")).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: /Clear selection/i }));
    await expect(canvas.getByText("0 selected")).toBeInTheDocument();
  },
};

export const ErrorState: Story = {
  args: {
    error: "internal_error: nodes query failed during snapshot aggregation",
  },
};

function buildInteractiveResponse(
  items: NodeListItem[],
  filterState: NodeFilterState,
): NodeListResponse {
  const filteredItems = items
    .filter((item) => matchesTextQuery(item, filterState.query))
    .filter((item) => matchesList(item.proxy_type, filterState.proxyTypes))
    .filter((item) => matchesList(item.country_code ?? "", filterState.countryCodes, true))
    .filter((item) => matchesList(item.region_name ?? "", filterState.regions))
    .filter((item) => matchesList(item.city ?? "", filterState.cities))
    .filter((item) => matchesProbeStatus(item, filterState.probeStatus))
    .filter((item) => matchesSessionPresence(item, filterState.sessionPresence))
    .filter((item) => matchesIpFamily(item, filterState.ipFamily))
    .sort((left, right) =>
      compareNodeItems(left, right, filterState.sortBy, filterState.sortOrder),
    );

  const pageSize = Math.max(1, filterState.pageSize);
  const total = filteredItems.length;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const page = Math.min(Math.max(filterState.page, 1), pageCount);
  const start = (page - 1) * pageSize;

  return {
    total,
    page,
    page_size: pageSize,
    items: filteredItems.slice(start, start + pageSize),
  };
}

function matchesTextQuery(item: NodeListItem, query: string) {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) {
    return true;
  }

  const haystack = [
    item.proxy_name,
    item.server,
    item.preferred_ip,
    item.ipv4,
    item.ipv6,
    item.country_code,
    item.country_name,
    item.region_name,
    item.city,
  ]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();

  return haystack.includes(normalizedQuery);
}

function matchesList(value: string, rawFilter: string, uppercase = false) {
  const values = splitFilterInput(rawFilter);
  if (values.length === 0) {
    return true;
  }

  const normalizedValue = uppercase ? value.toUpperCase() : value.toLowerCase();
  return values.some((entry) =>
    uppercase ? normalizedValue === entry.toUpperCase() : normalizedValue === entry.toLowerCase(),
  );
}

function matchesProbeStatus(item: NodeListItem, probeStatus: NodeFilterState["probeStatus"]) {
  return probeStatus === "any" ? true : item.probe_status === probeStatus;
}

function matchesSessionPresence(
  item: NodeListItem,
  sessionPresence: NodeFilterState["sessionPresence"],
) {
  if (sessionPresence === "with_sessions") {
    return item.session_count > 0;
  }
  if (sessionPresence === "without_sessions") {
    return item.session_count === 0;
  }
  return true;
}

function matchesIpFamily(item: NodeListItem, ipFamily: NodeFilterState["ipFamily"]) {
  switch (ipFamily) {
    case "ipv4":
      return Boolean(item.ipv4);
    case "ipv6":
      return Boolean(item.ipv6);
    case "dual_stack":
      return Boolean(item.ipv4 && item.ipv6);
    case "any":
      return true;
  }
}

function compareNodeItems(
  left: NodeListItem,
  right: NodeListItem,
  sortBy: NodeFilterState["sortBy"],
  sortOrder: NodeFilterState["sortOrder"],
) {
  const direction = sortOrder === "asc" ? 1 : -1;

  switch (sortBy) {
    case "proxy_name":
      return direction * compareText(left.proxy_name, right.proxy_name);
    case "proxy_type":
      return direction * compareText(left.proxy_type, right.proxy_type);
    case "preferred_ip":
      return direction * compareText(left.preferred_ip, right.preferred_ip);
    case "region":
      return direction * compareText(resolveRegionLabel(left), resolveRegionLabel(right));
    case "latency":
      return direction * compareNumber(left.best_latency_ms, right.best_latency_ms);
    case "last_used_at":
      return direction * compareNumber(left.last_used_at, right.last_used_at);
    case "session_count":
      return direction * compareNumber(left.session_count, right.session_count);
  }
}

function compareText(left?: string | null, right?: string | null) {
  return (left ?? "").localeCompare(right ?? "", undefined, { sensitivity: "base" });
}

function compareNumber(left?: number | null, right?: number | null) {
  if (left == null && right == null) {
    return 0;
  }
  if (left == null) {
    return 1;
  }
  if (right == null) {
    return -1;
  }
  return left - right;
}

function resolveRegionLabel(item: NodeListItem) {
  return [item.country_name ?? item.country_code, item.region_name, item.city]
    .filter(Boolean)
    .join(" / ");
}
