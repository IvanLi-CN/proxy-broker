import type {
  NodeExportFormat,
  NodeExportRequest,
  NodeIpFamilyFilter,
  NodeListItem,
  NodeListQuery,
  NodeOpenSessionsRequest,
  NodeProbeStatusFilter,
  NodeSessionPresenceFilter,
  NodeSortField,
  NodeViewMode,
  SortOrder,
} from "@/lib/types";

export interface NodeFilterState {
  query: string;
  proxyTypes: string;
  countryCodes: string;
  regions: string;
  cities: string;
  probeStatus: NodeProbeStatusFilter;
  sessionPresence: NodeSessionPresenceFilter;
  ipFamily: NodeIpFamilyFilter;
  sortBy: NodeSortField;
  sortOrder: SortOrder;
  page: number;
  pageSize: number;
}

export interface NodeGroup {
  key: string;
  label: string;
  items: NodeListItem[];
}

export const defaultNodeFilterState: NodeFilterState = {
  query: "",
  proxyTypes: "",
  countryCodes: "",
  regions: "",
  cities: "",
  probeStatus: "any",
  sessionPresence: "any",
  ipFamily: "any",
  sortBy: "proxy_name",
  sortOrder: "asc",
  page: 1,
  pageSize: 25,
};

export function splitFilterInput(value: string) {
  return value
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

export function buildNodeListQuery(state: NodeFilterState): NodeListQuery {
  return {
    query: state.query.trim() || undefined,
    proxy_types: listOrUndefined(state.proxyTypes),
    country_codes: listOrUndefined(state.countryCodes),
    regions: listOrUndefined(state.regions),
    cities: listOrUndefined(state.cities),
    probe_status: state.probeStatus,
    session_presence: state.sessionPresence,
    ip_family: state.ipFamily,
    sort_by: state.sortBy,
    sort_order: state.sortOrder,
    page: state.page,
    page_size: state.pageSize,
  };
}

export function groupNodeItems(items: NodeListItem[], viewMode: NodeViewMode): NodeGroup[] {
  if (viewMode === "flat") {
    return [{ key: "flat", label: "All nodes", items }];
  }

  const groups = new Map<string, NodeGroup>();

  for (const item of items) {
    const label = resolveNodeGroupLabel(item, viewMode);
    const key = `${viewMode}:${label}`;
    const current = groups.get(key);
    if (current) {
      current.items.push(item);
      continue;
    }
    groups.set(key, { key, label, items: [item] });
  }

  return Array.from(groups.values());
}

export function buildNodeExportRequest(
  scope: "selected" | "all_filtered",
  selectedIds: string[],
  query: NodeListQuery,
  format: NodeExportFormat,
): NodeExportRequest {
  return scope === "all_filtered"
    ? {
        all_filtered: true,
        query: { ...query, page: undefined, page_size: undefined },
        format,
      }
    : { node_ids: selectedIds, format };
}

export function buildNodeOpenSessionsRequest(
  scope: "selected" | "all_filtered",
  selectedIds: string[],
  query: NodeListQuery,
): NodeOpenSessionsRequest {
  return scope === "all_filtered"
    ? {
        all_filtered: true,
        query: { ...query, page: undefined, page_size: undefined },
        ip_family_priority: "ipv4_first",
      }
    : { node_ids: selectedIds, ip_family_priority: "ipv4_first" };
}

const selectionResetKeys: Array<keyof NodeFilterState> = [
  "query",
  "proxyTypes",
  "countryCodes",
  "regions",
  "cities",
  "probeStatus",
  "sessionPresence",
  "ipFamily",
];

export function shouldClearNodeSelectionForFilterPatch(patch: Partial<NodeFilterState>) {
  return selectionResetKeys.some((key) => Object.hasOwn(patch, key));
}

export function areAllPageNodesSelected(items: NodeListItem[], selectedIds: string[]) {
  return items.length > 0 && items.every((item) => selectedIds.includes(item.node_id));
}

function listOrUndefined(value: string) {
  const items = splitFilterInput(value);
  return items.length > 0 ? items : undefined;
}

function resolveNodeGroupLabel(item: NodeListItem, viewMode: NodeViewMode) {
  switch (viewMode) {
    case "group_by_ip":
      return item.preferred_ip ?? "No preferred IP";
    case "group_by_region":
      return (
        [item.country_name ?? item.country_code, item.region_name, item.city]
          .filter(Boolean)
          .join(" / ") || "Unknown region"
      );
    case "group_by_subscription":
      return item.subscription_value
        ? `${item.subscription_type}: ${item.subscription_value}`
        : item.subscription_type;
    case "flat":
      return "All nodes";
  }
}
