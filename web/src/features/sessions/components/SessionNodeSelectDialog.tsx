import {
  ActivityIcon,
  Globe2Icon,
  Layers3Icon,
  LoaderCircleIcon,
  PencilLineIcon,
} from "lucide-react";
import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { ProxyNodeLiveState } from "@/hooks/use-proxy-operation-events";
import { useI18n } from "@/i18n";
import {
  formatCountryName,
  formatGeoLabel,
  formatLatency,
  formatTimestamp,
  resolveSessionDisplayAddress,
} from "@/lib/format";
import {
  type ProbeDisplayState,
  probeLatencyBadgeToneClass,
  probeLatencyToneClass,
} from "@/lib/proxy-probe-display";
import type {
  SessionListItem,
  SessionNodeOptionItem,
  SessionNodeSortMode,
  UpdateSessionNodeRequest,
} from "@/lib/types";
import { cn } from "@/lib/utils";

interface SessionNodeSelectDialogProps {
  open: boolean;
  session: SessionListItem | null;
  isPending: boolean;
  error?: string | null;
  onOpenChange: (open: boolean) => void;
  onSearch: (
    sessionId: string,
    payload: { query?: string; sort_mode: SessionNodeSortMode; limit?: number },
  ) => Promise<SessionNodeOptionItem[] | undefined>;
  onSubmit: (sessionId: string, payload: UpdateSessionNodeRequest) => void | Promise<void>;
  onProbeNodes?: (nodeIds: string[]) => void | Promise<void>;
  probingNodeIds?: string[];
  liveNodeStates?: Record<string, ProxyNodeLiveState>;
  probeNodeStates?: Record<string, ProxyNodeLiveState>;
}

type GroupingMode = "geo" | "source";
type NodeGroupKind = "session_recent" | "project_recent" | "geo" | "source";

interface NodeGroup {
  key: string;
  kind: NodeGroupKind;
  label: string;
  count: number;
}

const SESSION_RECENT_GROUP_KEY = "special:session_recent";
const PROJECT_RECENT_GROUP_KEY = "special:project_recent";

function buildGeoSummary(
  locale: "zh-CN" | "en-US",
  item: Pick<SessionNodeOptionItem, "country_code" | "country_name" | "region_name" | "city">,
) {
  const geo = [
    formatCountryName(locale, item.country_code, item.country_name),
    formatGeoLabel(locale, item.region_name),
    formatGeoLabel(locale, item.city),
  ].filter(Boolean);
  return geo.join(" / ");
}

function compareNullableUsageDesc(left?: number | null, right?: number | null) {
  if (left != null && right != null && left !== right) {
    return right - left;
  }
  if (left != null && right == null) {
    return -1;
  }
  if (left == null && right != null) {
    return 1;
  }
  return 0;
}

function compareByStableName(left: SessionNodeOptionItem, right: SessionNodeOptionItem) {
  return (
    left.proxy_name.localeCompare(right.proxy_name) || left.node_id.localeCompare(right.node_id)
  );
}

function sortItemsForGroup(items: SessionNodeOptionItem[], groupKind: NodeGroupKind) {
  return [...items].sort((left, right) => {
    if (groupKind === "session_recent") {
      return (
        compareNullableUsageDesc(left.session_last_used_at, right.session_last_used_at) ||
        compareByStableName(left, right)
      );
    }
    return (
      compareNullableUsageDesc(left.project_last_used_at, right.project_last_used_at) ||
      compareByStableName(left, right)
    );
  });
}

function sourceGroupLabel(item: SessionNodeOptionItem, fallback: string) {
  const label = item.import_name?.trim() || item.source_label?.trim();
  return label || fallback;
}

function geoGroupLabel(locale: "zh-CN" | "en-US", item: SessionNodeOptionItem, fallback: string) {
  return buildGeoSummary(locale, item) || fallback;
}

function groupKey(kind: GroupingMode, label: string) {
  return `${kind}:${label}`;
}

export function SessionNodeSelectDialog({
  open,
  session,
  isPending,
  error,
  onOpenChange,
  onSearch,
  onSubmit,
  onProbeNodes,
  probingNodeIds = [],
  liveNodeStates = {},
  probeNodeStates = {},
}: SessionNodeSelectDialogProps) {
  const { locale, t } = useI18n();
  const displayAddress = session ? resolveSessionDisplayAddress(session) : null;
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const [groupingMode, setGroupingMode] = useState<GroupingMode>("geo");
  const [activeGroupKey, setActiveGroupKey] = useState(SESSION_RECENT_GROUP_KEY);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [items, setItems] = useState<SessionNodeOptionItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const requestVersion = useRef(0);
  const initializedSessionId = useRef<string | null>(null);

  useEffect(() => {
    if (!open || !session) {
      initializedSessionId.current = null;
      setSelectedNodeId(null);
      return;
    }
    if (initializedSessionId.current === session.session_id) {
      return;
    }
    initializedSessionId.current = session.session_id;
    setSelectedNodeId(session.node_id);
  }, [open, session]);

  useEffect(() => {
    if (!open || !session) {
      return;
    }

    const currentVersion = ++requestVersion.current;
    setLoading(true);
    setLoadError(null);

    void onSearch(session.session_id, {
      query: deferredQuery.trim() || undefined,
      sort_mode: "session_recent",
    })
      .then((nextItems) => {
        if (requestVersion.current !== currentVersion) {
          return;
        }
        setItems(nextItems ?? []);
      })
      .catch(() => {
        if (requestVersion.current !== currentVersion) {
          return;
        }
        setItems([]);
        setLoadError(t("Could not load node options"));
      })
      .finally(() => {
        if (requestVersion.current === currentVersion) {
          setLoading(false);
        }
      });
  }, [deferredQuery, onSearch, open, session, t]);

  const selectedItem = useMemo(
    () => items.find((item) => item.node_id === selectedNodeId) ?? null,
    [items, selectedNodeId],
  );
  const currentItem = useMemo(
    () => (session ? (items.find((item) => item.node_id === session.node_id) ?? null) : null),
    [items, session],
  );

  const grouped = useMemo(() => {
    const fallbackGeo = t("Unknown region");
    const fallbackSource = t("Unknown source");
    const generatedGroups = new Map<string, NodeGroup>();
    const itemsByGroup = new Map<string, SessionNodeOptionItem[]>();
    const modeKind: NodeGroupKind = groupingMode;

    for (const item of items) {
      const label =
        groupingMode === "geo"
          ? geoGroupLabel(locale, item, fallbackGeo)
          : sourceGroupLabel(item, fallbackSource);
      const key = groupKey(groupingMode, label);
      const previous = generatedGroups.get(key);
      generatedGroups.set(key, {
        key,
        kind: modeKind,
        label,
        count: (previous?.count ?? 0) + 1,
      });
      itemsByGroup.set(key, [...(itemsByGroup.get(key) ?? []), item]);
    }

    const specialGroups: NodeGroup[] = [
      {
        key: SESSION_RECENT_GROUP_KEY,
        kind: "session_recent",
        label: t("Current session last used"),
        count: items.length,
      },
      {
        key: PROJECT_RECENT_GROUP_KEY,
        kind: "project_recent",
        label: t("Current project last used"),
        count: items.length,
      },
    ];
    const regularGroups = [...generatedGroups.values()].sort(
      (left, right) => right.count - left.count || left.label.localeCompare(right.label),
    );
    return {
      groups: [...specialGroups, ...regularGroups],
      itemsByGroup,
    };
  }, [groupingMode, items, locale, t]);

  useEffect(() => {
    if (grouped.groups.some((group) => group.key === activeGroupKey)) {
      return;
    }
    setActiveGroupKey(SESSION_RECENT_GROUP_KEY);
  }, [activeGroupKey, grouped.groups]);

  const activeGroup =
    grouped.groups.find((group) => group.key === activeGroupKey) ?? grouped.groups[0] ?? null;
  const visibleItems = useMemo(() => {
    if (!activeGroup) {
      return [];
    }
    if (activeGroup.kind === "session_recent" || activeGroup.kind === "project_recent") {
      return sortItemsForGroup(items, activeGroup.kind);
    }
    return sortItemsForGroup(grouped.itemsByGroup.get(activeGroup.key) ?? [], activeGroup.kind);
  }, [activeGroup, grouped.itemsByGroup, items]);
  const visibleNodeIds = useMemo(
    () => [...new Set(visibleItems.map((item) => item.node_id))],
    [visibleItems],
  );
  const probingNodeIdSet = useMemo(() => new Set(probingNodeIds), [probingNodeIds]);
  const visibleGroupProbing = visibleNodeIds.some(
    (nodeId) =>
      probingNodeIdSet.has(nodeId) || liveNodeStates[nodeId]?.kind === "proxy_latency_probe",
  );
  const groupProbeDisabled = visibleNodeIds.length === 0 || !onProbeNodes || visibleGroupProbing;
  const currentNodeId = session?.node_id ?? null;
  const currentLiveState = currentNodeId ? liveNodeStates[currentNodeId] : undefined;
  const currentLiveProbeState =
    currentLiveState?.kind === "proxy_latency_probe" ? currentLiveState : null;
  const currentDisplayedState = currentNodeId
    ? (probeNodeStates[currentNodeId] ?? currentLiveState)
    : undefined;
  const currentDisplayedProbeState =
    currentDisplayedState?.kind === "proxy_latency_probe" ? currentDisplayedState : null;
  const currentRecentSamples = currentItem?.recent_probe_samples ?? [];
  const currentLatestSample = currentRecentSamples[0] ?? null;
  const currentLiveLatency =
    currentDisplayedProbeState?.latestSampleMs == null
      ? null
      : formatLatency(locale, t, currentDisplayedProbeState.latestSampleMs);
  const currentProbeState: ProbeDisplayState = currentDisplayedProbeState
    ? currentDisplayedProbeState.latestSampleMs == null
      ? "failed"
      : "success"
    : currentLatestSample
      ? currentLatestSample.ok && currentLatestSample.latency_ms != null
        ? "success"
        : "failed"
      : "empty";
  const currentLatency = currentDisplayedProbeState
    ? (currentDisplayedProbeState.latestSampleMs ?? null)
    : currentLatestSample?.ok
      ? (currentLatestSample.latency_ms ?? null)
      : null;
  const currentProbing =
    Boolean(currentNodeId && probingNodeIdSet.has(currentNodeId)) || Boolean(currentLiveProbeState);
  const currentProbeDisabled = !currentNodeId || !onProbeNodes || currentProbing;
  const currentGeoSummary = session ? buildGeoSummary(locale, currentItem ?? session) : "";

  const handleOpenChange = (nextOpen: boolean) => {
    onOpenChange(nextOpen);
    if (!nextOpen) {
      setQuery("");
      setGroupingMode("geo");
      setActiveGroupKey(SESSION_RECENT_GROUP_KEY);
      setItems([]);
      setLoadError(null);
      setSelectedNodeId(session?.node_id ?? null);
    }
  };

  const submitDisabled =
    !session || !selectedNodeId || selectedNodeId === session.node_id || isPending;

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="flex h-[calc(100dvh-1rem)] max-h-[calc(100dvh-1rem)] w-[calc(100vw-1rem)] max-w-[calc(100vw-1rem)] flex-col overflow-hidden sm:h-[calc(100dvh-3rem)] sm:max-h-[860px] sm:w-[calc(100vw-3rem)] sm:max-w-[1180px] lg:max-w-[1240px]">
        <DialogHeader>
          <DialogTitle>{t("Switch session proxy")}</DialogTitle>
          <DialogDescription>
            {session
              ? t(
                  "Pick a new node for {sessionId}. The session keeps the same listener and port.",
                  {
                    sessionId: session.session_id,
                  },
                )
              : t("Select a session before switching its node.")}
          </DialogDescription>
        </DialogHeader>

        {session ? (
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border/70 bg-muted/20 px-4 py-3 text-sm">
            <div className="min-w-0">
              <div className="truncate font-medium text-foreground">{session.proxy_name}</div>
              <div className="mt-1 flex flex-wrap gap-2 text-xs text-muted-foreground">
                <span>{t("Session ID: {sessionId}", { sessionId: session.session_id })}</span>
                <span>{t("Address {address}", { address: displayAddress ?? session.listen })}</span>
                <span>{t("Selected IP {ip}", { ip: session.selected_ip })}</span>
                {currentGeoSummary ? <span>{currentGeoSummary}</span> : null}
              </div>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <Tooltip>
                <TooltipTrigger asChild>
                  <span
                    className={cn(
                      "cursor-help text-xs font-semibold underline decoration-dotted underline-offset-4",
                      probeLatencyToneClass(currentProbeState, currentLatency),
                    )}
                  >
                    {currentDisplayedProbeState
                      ? (currentLiveLatency ?? t("Probe failed"))
                      : currentLatestSample
                        ? currentLatestSample.ok && currentLatestSample.latency_ms != null
                          ? formatLatency(locale, t, currentLatestSample.latency_ms)
                          : t("Probe failed")
                        : t("-- ms")}
                  </span>
                </TooltipTrigger>
                <TooltipContent
                  side="top"
                  align="end"
                  className="max-w-sm border border-border bg-popover text-popover-foreground shadow-xl shadow-foreground/10"
                  arrowClassName="bg-popover fill-popover"
                >
                  {currentRecentSamples.length > 0 ? (
                    <div className="space-y-1">
                      {currentRecentSamples.slice(0, 10).map((sample) => (
                        <div
                          key={`${sample.node_id}-${sample.ip}-${sample.sampled_at}-${sample.target_url}-${sample.ok ? sample.latency_ms : "fail"}`}
                          className="flex min-w-52 justify-between gap-4"
                        >
                          <span>{formatTimestamp(locale, t, sample.sampled_at)}</span>
                          <span
                            className={cn(
                              "rounded-md border px-1.5 py-0.5 font-mono font-semibold",
                              probeLatencyBadgeToneClass(
                                sample.ok && sample.latency_ms != null ? "success" : "failed",
                                sample.latency_ms,
                              ),
                            )}
                          >
                            {sample.ok && sample.latency_ms != null
                              ? formatLatency(locale, t, sample.latency_ms)
                              : t("Probe failed")}
                          </span>
                        </div>
                      ))}
                    </div>
                  ) : (
                    t("No probe data yet")
                  )}
                </TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="inline-flex">
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-sm"
                      className="size-11 rounded-full border-transparent bg-transparent p-0 hover:bg-transparent"
                      disabled={currentProbeDisabled}
                      aria-label={t("Probe current node {name}", { name: session.proxy_name })}
                      onClick={() => {
                        if (!onProbeNodes || !currentNodeId) {
                          return;
                        }
                        void onProbeNodes([currentNodeId]);
                      }}
                    >
                      <span className="inline-flex size-7 items-center justify-center rounded-[min(var(--radius-md),12px)] border border-border bg-background text-foreground shadow-sm transition-colors group-hover/button:bg-muted">
                        {currentProbing ? (
                          <LoaderCircleIcon className="size-3.5 animate-spin" />
                        ) : (
                          <ActivityIcon className="size-3.5" />
                        )}
                      </span>
                    </Button>
                  </span>
                </TooltipTrigger>
                <TooltipContent>
                  {currentProbing
                    ? t("This node is already queued or running a probe")
                    : t("Probe this current node")}
                </TooltipContent>
              </Tooltip>
            </div>
          </div>
        ) : null}

        <div className="space-y-2">
          <Label htmlFor="session-node-query">{t("Filter nodes")}</Label>
          <Input
            id="session-node-query"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("Search by node, source, IP, or location")}
          />
        </div>

        <div className="grid min-h-0 flex-1 gap-4 overflow-hidden md:grid-cols-[300px_minmax(0,1fr)]">
          <div className="flex min-h-0 flex-col gap-2">
            <div className="flex items-center justify-between gap-3">
              <Label>{t("Group nodes")}</Label>
              <ToggleGroup
                type="single"
                value={groupingMode}
                onValueChange={(value) => {
                  if (value === "geo" || value === "source") {
                    setGroupingMode(value);
                    setActiveGroupKey(SESSION_RECENT_GROUP_KEY);
                  }
                }}
                variant="outline"
                size="sm"
                aria-label={t("Group nodes")}
                className="bg-card"
              >
                <ToggleGroupItem value="geo" aria-label={t("Group by region")}>
                  <Globe2Icon className="size-4" />
                  <span className="hidden sm:inline">{t("Region")}</span>
                </ToggleGroupItem>
                <ToggleGroupItem value="source" aria-label={t("Group by subscription")}>
                  <Layers3Icon className="size-4" />
                  <span className="hidden sm:inline">{t("Source")}</span>
                </ToggleGroupItem>
              </ToggleGroup>
            </div>
            <ScrollArea className="h-48 rounded-xl border border-border/70 bg-card/70 md:h-full">
              <div className="space-y-1 p-2">
                {grouped.groups.map((group) => {
                  const active = group.key === activeGroupKey;
                  return (
                    <button
                      key={group.key}
                      type="button"
                      onClick={() => setActiveGroupKey(group.key)}
                      className={cn(
                        "flex w-full items-center justify-between gap-3 rounded-lg px-3 py-2 text-left text-sm transition-colors",
                        active
                          ? "bg-primary text-primary-foreground shadow-sm"
                          : "text-muted-foreground hover:bg-muted hover:text-foreground",
                      )}
                    >
                      <span className="min-w-0">
                        <span className="block truncate font-medium">{group.label}</span>
                        <span
                          className={cn(
                            "block truncate text-xs",
                            active ? "text-primary-foreground/75" : "text-muted-foreground",
                          )}
                        >
                          {t("{available} / {total} available", {
                            available: group.count,
                            total: items.length,
                          })}
                        </span>
                      </span>
                      <Badge
                        variant={active ? "secondary" : "outline"}
                        className="shrink-0 tabular-nums"
                      >
                        {group.count}
                      </Badge>
                    </button>
                  );
                })}
              </div>
            </ScrollArea>
          </div>

          <div className="flex min-h-0 min-w-0 flex-col gap-3">
            <div className="flex min-h-9 flex-wrap items-center justify-between gap-3">
              <div className="min-w-0">
                <div className="truncate text-sm font-semibold text-foreground">
                  {activeGroup?.label ?? t("No matching nodes")}
                </div>
                <div className="text-xs text-muted-foreground">
                  {t("{count} nodes", { count: visibleItems.length })}
                </div>
              </div>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="inline-flex">
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      disabled={groupProbeDisabled}
                      onClick={() => {
                        if (!onProbeNodes || visibleNodeIds.length === 0) {
                          return;
                        }
                        void onProbeNodes(visibleNodeIds);
                      }}
                    >
                      {visibleGroupProbing ? (
                        <LoaderCircleIcon className="size-3.5 animate-spin" />
                      ) : (
                        <ActivityIcon className="size-3.5" />
                      )}
                      {t("Probe current group")}
                    </Button>
                  </span>
                </TooltipTrigger>
                <TooltipContent>
                  {visibleGroupProbing
                    ? t("A visible node in this group is already queued or running a probe")
                    : t("Probe all visible nodes in this group")}
                </TooltipContent>
              </Tooltip>
            </div>
            <ScrollArea className="min-h-0 flex-1 pr-3">
              <div className="space-y-3 pr-3">
                {loading ? (
                  <div className="flex items-center justify-center gap-2 rounded-xl border border-dashed border-border/80 px-4 py-12 text-sm text-muted-foreground">
                    <LoaderCircleIcon className="size-4 animate-spin" />
                    {t("Loading node options…")}
                  </div>
                ) : null}
                {!loading && loadError ? (
                  <div className="rounded-xl border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive">
                    {loadError}
                  </div>
                ) : null}
                {!loading && !loadError && visibleItems.length === 0 ? (
                  <div className="rounded-xl border border-dashed border-border/80 px-4 py-12 text-sm text-muted-foreground">
                    {t("No matching nodes")}
                  </div>
                ) : null}
                {!loading && !loadError
                  ? visibleItems.map((item) => {
                      const selected = item.node_id === selectedNodeId;
                      const current = item.node_id === session?.node_id;
                      const geoSummary = buildGeoSummary(locale, item);
                      const metaParts = [item.source_label, item.primary_ip, geoSummary].filter(
                        Boolean,
                      );
                      const recentSamples = item.recent_probe_samples ?? [];
                      const latestSample = recentSamples[0] ?? null;
                      const liveState = liveNodeStates[item.node_id];
                      const liveProbeState =
                        liveState?.kind === "proxy_latency_probe" ? liveState : null;
                      const displayedState = probeNodeStates[item.node_id] ?? liveState;
                      const displayedProbeState =
                        displayedState?.kind === "proxy_latency_probe" ? displayedState : null;
                      const liveLatency =
                        displayedProbeState?.latestSampleMs == null
                          ? null
                          : formatLatency(locale, t, displayedProbeState.latestSampleMs);
                      const probeState: ProbeDisplayState = displayedProbeState
                        ? displayedProbeState.latestSampleMs == null
                          ? "failed"
                          : "success"
                        : latestSample
                          ? latestSample.ok && latestSample.latency_ms != null
                            ? "success"
                            : "failed"
                          : "empty";
                      const latency = displayedProbeState
                        ? (displayedProbeState.latestSampleMs ?? null)
                        : latestSample?.ok
                          ? (latestSample.latency_ms ?? null)
                          : null;
                      const probing = probingNodeIdSet.has(item.node_id) || Boolean(liveProbeState);
                      return (
                        <div key={item.node_id} className="relative">
                          <button
                            type="button"
                            aria-label={t("Select node {name}", { name: item.proxy_name })}
                            onClick={() => setSelectedNodeId(item.node_id)}
                            className={cn(
                              "w-full rounded-xl border px-4 py-3 text-left transition-colors",
                              selected
                                ? "border-primary bg-primary/5 shadow-sm"
                                : "border-border/70 bg-background hover:bg-muted/30",
                            )}
                          >
                            <div className="flex flex-wrap items-start justify-between gap-3">
                              <div className="min-w-0 space-y-1">
                                <div className="flex flex-wrap items-center gap-2">
                                  <div className="font-medium text-foreground">
                                    {item.proxy_name}
                                  </div>
                                  {current ? <Badge variant="outline">{t("Current")}</Badge> : null}
                                  {selected && !current ? <Badge>{t("Selected")}</Badge> : null}
                                </div>
                                {metaParts.length > 0 ? (
                                  <div className="text-xs text-muted-foreground">
                                    {metaParts.join(" · ")}
                                  </div>
                                ) : null}
                              </div>
                              <div className="grid shrink-0 gap-1 text-right text-xs text-muted-foreground">
                                <span>
                                  {t("Session last used {time}", {
                                    time: formatTimestamp(locale, t, item.session_last_used_at),
                                  })}
                                </span>
                                <span>
                                  {t("Project last used {time}", {
                                    time: formatTimestamp(locale, t, item.project_last_used_at),
                                  })}
                                </span>
                              </div>
                            </div>
                            <div className="mt-3 flex flex-wrap gap-2 text-xs text-muted-foreground">
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <span
                                    className={cn(
                                      "cursor-help font-semibold underline decoration-dotted underline-offset-4",
                                      probeLatencyToneClass(probeState, latency),
                                    )}
                                  >
                                    {displayedProbeState
                                      ? (liveLatency ?? t("Probe failed"))
                                      : latestSample
                                        ? latestSample.ok && latestSample.latency_ms != null
                                          ? formatLatency(locale, t, latestSample.latency_ms)
                                          : t("Probe failed")
                                        : t("-- ms")}
                                  </span>
                                </TooltipTrigger>
                                <TooltipContent
                                  side="top"
                                  align="start"
                                  className="max-w-sm border border-border bg-popover text-popover-foreground shadow-xl shadow-foreground/10"
                                  arrowClassName="bg-popover fill-popover"
                                >
                                  {recentSamples.length > 0 ? (
                                    <div className="space-y-1">
                                      {recentSamples.slice(0, 10).map((sample) => (
                                        <div
                                          key={`${sample.node_id}-${sample.ip}-${sample.sampled_at}-${sample.target_url}-${sample.ok ? sample.latency_ms : "fail"}`}
                                          className="flex min-w-52 justify-between gap-4"
                                        >
                                          <span>
                                            {formatTimestamp(locale, t, sample.sampled_at)}
                                          </span>
                                          <span
                                            className={cn(
                                              "rounded-md border px-1.5 py-0.5 font-mono font-semibold",
                                              probeLatencyBadgeToneClass(
                                                sample.ok && sample.latency_ms != null
                                                  ? "success"
                                                  : "failed",
                                                sample.latency_ms,
                                              ),
                                            )}
                                          >
                                            {sample.ok && sample.latency_ms != null
                                              ? formatLatency(locale, t, sample.latency_ms)
                                              : t("Probe failed")}
                                          </span>
                                        </div>
                                      ))}
                                    </div>
                                  ) : (
                                    t("No probe data yet")
                                  )}
                                </TooltipContent>
                              </Tooltip>
                              {displayedProbeState?.message ? (
                                <span className="min-w-0 max-w-[calc(100%-3rem)] truncate">
                                  {displayedProbeState.message}
                                </span>
                              ) : item.import_name ? (
                                <span className="min-w-0 max-w-[calc(100%-3rem)] truncate">
                                  {item.import_name}
                                </span>
                              ) : null}
                            </div>
                          </button>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <span className="absolute right-2 bottom-1 inline-flex size-11 items-center justify-center">
                                <Button
                                  type="button"
                                  variant="outline"
                                  size="icon-sm"
                                  disabled={!onProbeNodes || probing}
                                  aria-label={t("Probe node {name}", { name: item.proxy_name })}
                                  onClick={(event) => {
                                    event.stopPropagation();
                                    if (!onProbeNodes) {
                                      return;
                                    }
                                    void onProbeNodes([item.node_id]);
                                  }}
                                >
                                  {probing ? (
                                    <LoaderCircleIcon className="size-3.5 animate-spin" />
                                  ) : (
                                    <ActivityIcon className="size-3.5" />
                                  )}
                                </Button>
                              </span>
                            </TooltipTrigger>
                            <TooltipContent>
                              {probing
                                ? t("This node is already queued or running a probe")
                                : t("Probe this node")}
                            </TooltipContent>
                          </Tooltip>
                        </div>
                      );
                    })
                  : null}
              </div>
            </ScrollArea>
          </div>
        </div>

        {error ? (
          <div className="rounded-xl border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive">
            {error}
          </div>
        ) : null}

        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)}>
            {t("Cancel")}
          </Button>
          <Button
            onClick={() => {
              if (!session || !selectedNodeId) {
                return;
              }
              void onSubmit(session.session_id, { node_id: selectedNodeId });
            }}
            disabled={submitDisabled}
          >
            {isPending ? (
              <>
                <LoaderCircleIcon className="mr-2 size-4 animate-spin" />
                {t("Switching proxy…")}
              </>
            ) : (
              <>
                <PencilLineIcon className="mr-2 size-4" />
                {t("Use selected node")}
              </>
            )}
          </Button>
        </DialogFooter>

        {selectedItem && selectedItem.node_id !== session?.node_id ? (
          <div className="text-xs text-muted-foreground">
            {t("Switch to {proxyName} via {primaryIp}", {
              proxyName: selectedItem.proxy_name,
              primaryIp: selectedItem.primary_ip ?? t("No resolved IPs"),
            })}
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
