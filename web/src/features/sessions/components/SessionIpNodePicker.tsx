import {
  CheckIcon,
  ChevronRightIcon,
  Clock3Icon,
  LoaderCircleIcon,
  NetworkIcon,
  SearchIcon,
} from "lucide-react";
import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useI18n } from "@/i18n";
import { formatCountryName, formatGeoLabel, formatLatency, formatTimestamp } from "@/lib/format";
import type {
  SearchSessionIpNodeOptionsRequest,
  SessionIpNodeGroupBy,
  SessionIpNodeOptionGroupItem,
  SessionIpNodeOptionIpItem,
  SessionIpNodeOptionNodeItem,
} from "@/lib/types";
import { cn } from "@/lib/utils";

export interface SessionIpNodePickerSelection {
  selectedIp: string;
  candidateNodeIds: string[];
}

interface SessionIpNodePickerProps {
  mode: "single" | "multiple";
  sessionId?: string;
  initialSelectedIp?: string | null;
  initialCandidateNodeIds?: string[];
  disabled?: boolean;
  onSelectionChange: (selections: SessionIpNodePickerSelection[]) => void;
  onSearch: (
    payload: SearchSessionIpNodeOptionsRequest,
  ) => Promise<SessionIpNodeOptionGroupItem[] | undefined>;
}

function relativeTime(
  locale: "zh-CN" | "en-US",
  t: (message: string) => string,
  epoch?: number | null,
) {
  if (!epoch) {
    return t("Never");
  }
  const deltaSeconds = Math.max(0, Math.floor(Date.now() / 1000) - epoch);
  const units: Array<[Intl.RelativeTimeFormatUnit, number]> = [
    ["day", 86_400],
    ["hour", 3_600],
    ["minute", 60],
  ];
  const [unit, seconds] = units.find(([, seconds]) => deltaSeconds >= seconds) ?? ["second", 1];
  const value = Math.max(1, Math.floor(deltaSeconds / seconds));
  return new Intl.RelativeTimeFormat(locale, { numeric: "auto" }).format(-value, unit);
}

function geoSummary(locale: "zh-CN" | "en-US", item: SessionIpNodeOptionIpItem) {
  return [
    formatCountryName(locale, item.country_code, item.country_name),
    formatGeoLabel(locale, item.region_name),
    formatGeoLabel(locale, item.city),
  ]
    .filter(Boolean)
    .join(" / ");
}

function nodeGeoSummary(locale: "zh-CN" | "en-US", item: SessionIpNodeOptionNodeItem) {
  return [
    formatCountryName(locale, item.country_code, item.country_name),
    formatGeoLabel(locale, item.region_name),
    formatGeoLabel(locale, item.city),
  ]
    .filter(Boolean)
    .join(" / ");
}

function flattenGroups(groups: SessionIpNodeOptionGroupItem[]) {
  return groups.flatMap((group) => group.items);
}

function defaultNodeIds(item: SessionIpNodeOptionIpItem) {
  return item.nodes.map((node) => node.node_id);
}

type LatencyQuality = "excellent" | "good" | "fair" | "poor" | "failed" | "unknown";

function latencyQuality(latency?: number | null, ok?: boolean | null): LatencyQuality {
  if (ok === false) {
    return "failed";
  }
  if (latency == null) {
    return "unknown";
  }
  if (latency <= 100) {
    return "excellent";
  }
  if (latency <= 200) {
    return "good";
  }
  if (latency <= 1000) {
    return "fair";
  }
  return "poor";
}

function latencyQualityClass(quality: LatencyQuality) {
  switch (quality) {
    case "excellent":
      return "border-emerald-500/25 bg-emerald-500/[0.12] text-emerald-700 dark:text-emerald-300";
    case "good":
      return "border-sky-500/25 bg-sky-500/[0.12] text-sky-700 dark:text-sky-300";
    case "fair":
      return "border-amber-500/25 bg-amber-500/[0.12] text-amber-700 dark:text-amber-300";
    case "poor":
      return "border-rose-500/25 bg-rose-500/[0.1] text-rose-700 dark:text-rose-300";
    case "failed":
      return "border-destructive/25 bg-destructive/10 text-destructive";
    case "unknown":
      return "border-border bg-muted/60 text-muted-foreground";
  }
}

function latencyQualityLabel(t: (message: string) => string, quality: LatencyQuality) {
  switch (quality) {
    case "excellent":
      return t("Excellent");
    case "good":
      return t("Good");
    case "fair":
      return t("Fair");
    case "poor":
      return t("Poor");
    case "failed":
      return t("Failed");
    case "unknown":
      return t("Unknown");
  }
}

function LatencyBadge({
  latency,
  probeOk,
  label,
}: {
  latency?: number | null;
  probeOk?: boolean | null;
  label: string;
}) {
  const quality = latencyQuality(latency, probeOk);
  return (
    <span
      className={cn(
        "inline-flex h-6 min-w-16 items-center justify-center rounded-full border px-2 text-xs font-semibold tabular-nums",
        latencyQualityClass(quality),
      )}
    >
      {label}
    </span>
  );
}

function SelectionMark({ checked }: { checked: boolean }) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "flex size-4 shrink-0 items-center justify-center rounded-[4px] border transition-colors",
        checked
          ? "border-primary bg-primary text-primary-foreground"
          : "border-input bg-background",
      )}
    >
      {checked ? <CheckIcon className="size-3" /> : null}
    </span>
  );
}

export function SessionIpNodePicker({
  mode,
  sessionId,
  initialSelectedIp,
  initialCandidateNodeIds = [],
  disabled = false,
  onSelectionChange,
  onSearch,
}: SessionIpNodePickerProps) {
  const { locale, t } = useI18n();
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const [groupBy, setGroupBy] = useState<SessionIpNodeGroupBy>("subscription");
  const [groups, setGroups] = useState<SessionIpNodeOptionGroupItem[]>([]);
  const [activeIp, setActiveIp] = useState<string | null>(initialSelectedIp ?? null);
  const [selectedIps, setSelectedIps] = useState<Set<string>>(
    () => new Set(initialSelectedIp ? [initialSelectedIp] : []),
  );
  const [candidateByIp, setCandidateByIp] = useState<Map<string, Set<string>>>(() => {
    const map = new Map<string, Set<string>>();
    if (initialSelectedIp && initialCandidateNodeIds.length > 0) {
      map.set(initialSelectedIp, new Set(initialCandidateNodeIds));
    }
    return map;
  });
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const requestVersion = useRef(0);
  const lastSelectionKey = useRef("");

  useEffect(() => {
    const currentVersion = ++requestVersion.current;
    setLoading(true);
    setLoadError(null);
    void onSearch({
      query: deferredQuery.trim() || undefined,
      group_by: groupBy,
      session_id: sessionId,
      limit: 80,
    })
      .then((nextGroups) => {
        if (requestVersion.current !== currentVersion) {
          return;
        }
        setGroups(nextGroups ?? []);
      })
      .catch(() => {
        if (requestVersion.current !== currentVersion) {
          return;
        }
        setGroups([]);
        setLoadError(t("Could not load IP options"));
      })
      .finally(() => {
        if (requestVersion.current === currentVersion) {
          setLoading(false);
        }
      });
  }, [deferredQuery, groupBy, onSearch, sessionId, t]);

  const items = useMemo(() => flattenGroups(groups), [groups]);
  const itemByIp = useMemo(() => new Map(items.map((item) => [item.ip, item])), [items]);
  const activeItem = activeIp ? (itemByIp.get(activeIp) ?? null) : null;

  useEffect(() => {
    if (activeIp || items.length === 0) {
      return;
    }
    const first = items[0];
    if (!first) {
      return;
    }
    setActiveIp(first.ip);
    setSelectedIps(new Set([first.ip]));
    setCandidateByIp((current) => new Map(current).set(first.ip, new Set(defaultNodeIds(first))));
  }, [activeIp, items]);

  useEffect(() => {
    const selections = [...selectedIps]
      .map((selectedIp) => {
        const item = itemByIp.get(selectedIp);
        const candidateNodeIds = [
          ...(candidateByIp.get(selectedIp) ?? new Set(item ? defaultNodeIds(item) : [])),
        ];
        return { selectedIp, candidateNodeIds };
      })
      .filter((selection) => selection.candidateNodeIds.length > 0);
    const nextSelectionKey = selections
      .map((selection) => `${selection.selectedIp}:${selection.candidateNodeIds.join(",")}`)
      .sort()
      .join("|");
    if (nextSelectionKey === lastSelectionKey.current) {
      return;
    }
    lastSelectionKey.current = nextSelectionKey;
    onSelectionChange(selections);
  }, [candidateByIp, itemByIp, onSelectionChange, selectedIps]);

  const toggleIp = (item: SessionIpNodeOptionIpItem) => {
    setActiveIp(item.ip);
    setCandidateByIp((current) => {
      const next = new Map(current);
      if (!next.has(item.ip)) {
        next.set(item.ip, new Set(defaultNodeIds(item)));
      }
      return next;
    });
    setSelectedIps((current) => {
      if (mode === "single") {
        return new Set([item.ip]);
      }
      const next = new Set(current);
      if (next.has(item.ip)) {
        next.delete(item.ip);
      } else {
        next.add(item.ip);
      }
      return next;
    });
  };

  const toggleNode = (nodeId: string) => {
    if (!activeItem || disabled) {
      return;
    }
    setCandidateByIp((current) => {
      const next = new Map(current);
      const currentSet = new Set(next.get(activeItem.ip) ?? defaultNodeIds(activeItem));
      if (currentSet.has(nodeId)) {
        currentSet.delete(nodeId);
      } else {
        currentSet.add(nodeId);
      }
      next.set(activeItem.ip, currentSet);
      return next;
    });
    setSelectedIps((current) => new Set(current).add(activeItem.ip));
  };

  const selectedNodeIds = activeItem
    ? (candidateByIp.get(activeItem.ip) ?? new Set(defaultNodeIds(activeItem)))
    : new Set<string>();

  return (
    <TooltipProvider>
      <div className="grid min-h-[560px] gap-4 lg:grid-cols-[minmax(480px,1.15fr)_minmax(320px,0.85fr)]">
        <div className="min-w-0 rounded-lg border border-border/70 bg-background">
          <div className="flex flex-col gap-3 border-b border-border/70 p-3 md:flex-row md:items-center">
            <div className="relative min-w-0 flex-1">
              <SearchIcon className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                className="pl-9"
                placeholder={t("Search IP, node, subscription, or city")}
                disabled={disabled}
              />
            </div>
            <ToggleGroup
              type="single"
              value={groupBy}
              onValueChange={(value) => value && setGroupBy(value as SessionIpNodeGroupBy)}
              variant="outline"
              size="sm"
              className="w-fit"
            >
              <ToggleGroupItem value="subscription">{t("Subscription")}</ToggleGroupItem>
              <ToggleGroupItem value="city">{t("City")}</ToggleGroupItem>
            </ToggleGroup>
          </div>

          <div className="max-h-[500px] overflow-auto">
            {loading ? (
              <div className="flex items-center justify-center gap-2 px-4 py-14 text-sm text-muted-foreground">
                <LoaderCircleIcon className="size-4 animate-spin" />
                {t("Loading IP options...")}
              </div>
            ) : null}
            {!loading && loadError ? (
              <div className="px-4 py-10 text-sm text-destructive">{loadError}</div>
            ) : null}
            {!loading && !loadError && groups.length === 0 ? (
              <div className="px-4 py-10 text-sm text-muted-foreground">{t("No matching IPs")}</div>
            ) : null}
            {!loading && !loadError
              ? groups.map((group) => (
                  <div key={group.key}>
                    <div className="sticky top-0 z-10 border-y border-border/70 bg-muted/60 px-3 py-2 text-xs font-medium text-muted-foreground">
                      {group.label}
                    </div>
                    {group.items.map((item) => {
                      const selected = selectedIps.has(item.ip);
                      const active = item.ip === activeIp;
                      const summary = geoSummary(locale, item);
                      return (
                        <button
                          key={item.ip}
                          type="button"
                          onClick={() => toggleIp(item)}
                          disabled={disabled}
                          className={cn(
                            "grid w-full cursor-pointer grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-3 gap-y-1 border-b border-border/60 px-3 py-3 text-left text-sm transition-colors md:grid-cols-[auto_minmax(120px,1fr)_minmax(130px,0.8fr)_120px_90px_auto] md:gap-3",
                            active ? "bg-primary/5" : "hover:bg-muted/40",
                            disabled && "pointer-events-none opacity-60",
                          )}
                        >
                          <div className="row-span-4 self-start pt-0.5 md:row-auto md:self-center md:pt-0">
                            <SelectionMark checked={selected} />
                          </div>
                          <div className="col-start-2 row-start-1 min-w-0 md:col-auto md:row-auto">
                            <div className="truncate font-medium text-foreground">{item.ip}</div>
                            <div className="truncate text-xs text-muted-foreground">
                              {item.nodes.length} {t("nodes")}
                            </div>
                          </div>
                          <div className="col-start-2 row-start-2 min-w-0 truncate text-xs text-muted-foreground md:col-auto md:row-auto">
                            {groupBy === "subscription"
                              ? summary || t("Unknown location")
                              : item.subscription_name || t("Unknown subscription")}
                          </div>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <div className="col-start-2 row-start-3 truncate text-xs text-muted-foreground md:col-auto md:row-auto">
                                {relativeTime(locale, t, item.last_used_at)}
                              </div>
                            </TooltipTrigger>
                            <TooltipContent>
                              {formatTimestamp(locale, t, item.last_used_at)}
                            </TooltipContent>
                          </Tooltip>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <div className="col-start-2 row-start-4 mt-1 md:col-auto md:row-auto md:mt-0">
                                <LatencyBadge
                                  latency={item.best_latency_ms}
                                  label={formatLatency(locale, t, item.best_latency_ms)}
                                />
                              </div>
                            </TooltipTrigger>
                            <TooltipContent>
                              {t("Latency quality: {quality}", {
                                quality: latencyQualityLabel(
                                  t,
                                  latencyQuality(item.best_latency_ms),
                                ),
                              })}
                            </TooltipContent>
                          </Tooltip>
                          <ChevronRightIcon className="col-start-3 row-start-1 size-4 justify-self-end text-muted-foreground md:col-auto md:row-auto md:justify-self-auto" />
                        </button>
                      );
                    })}
                  </div>
                ))
              : null}
          </div>
        </div>

        <div className="min-w-0 rounded-lg border border-border/70 bg-background">
          <div className="border-b border-border/70 p-4">
            <div className="flex items-center gap-2 text-sm font-medium">
              <NetworkIcon className="size-4" />
              {activeItem ? activeItem.ip : t("Select an IP")}
            </div>
            <div className="mt-1 text-xs text-muted-foreground">
              {activeItem
                ? t("{count} candidate nodes selected", { count: selectedNodeIds.size })
                : t("Pick an IP on the left to review candidate nodes.")}
            </div>
          </div>
          <div className="max-h-[500px] space-y-2 overflow-auto p-3">
            {activeItem
              ? activeItem.nodes.map((node) => {
                  const checked = selectedNodeIds.has(node.node_id);
                  const meta = [node.source_label, nodeGeoSummary(locale, node)].filter(Boolean);
                  return (
                    <button
                      key={node.node_id}
                      type="button"
                      onClick={() => toggleNode(node.node_id)}
                      disabled={disabled}
                      className={cn(
                        "w-full cursor-pointer rounded-lg border px-3 py-3 text-left transition-colors",
                        checked
                          ? "border-primary bg-primary/5"
                          : "border-border/70 hover:bg-muted/40",
                        disabled && "pointer-events-none opacity-60",
                      )}
                    >
                      <div className="flex items-start gap-3">
                        <SelectionMark checked={checked} />
                        <div className="min-w-0 flex-1">
                          <div className="flex flex-wrap items-center gap-2">
                            <span className="truncate font-medium text-foreground">
                              {node.proxy_name}
                            </span>
                            {checked ? (
                              <Badge variant="outline" className="gap-1">
                                <CheckIcon className="size-3" />
                                {t("Candidate")}
                              </Badge>
                            ) : null}
                          </div>
                          {meta.length > 0 ? (
                            <div className="mt-1 truncate text-xs text-muted-foreground">
                              {meta.join(" · ")}
                            </div>
                          ) : null}
                          <div className="mt-3 flex flex-wrap gap-2 text-xs text-muted-foreground">
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <div>
                                  <LatencyBadge
                                    latency={node.median_latency_ms}
                                    probeOk={node.last_probe_ok}
                                    label={
                                      node.last_probe_ok === false
                                        ? t("Probe failed")
                                        : node.last_probe_ok === true
                                          ? formatLatency(locale, t, node.median_latency_ms)
                                          : t("No probe data yet")
                                    }
                                  />
                                </div>
                              </TooltipTrigger>
                              <TooltipContent>
                                {node.last_probe_ok === false
                                  ? t("Last probe failed")
                                  : t("Median latency {latency}. Quality: {quality}", {
                                      latency: formatLatency(locale, t, node.median_latency_ms),
                                      quality: latencyQualityLabel(
                                        t,
                                        latencyQuality(node.median_latency_ms, node.last_probe_ok),
                                      ),
                                    })}
                              </TooltipContent>
                            </Tooltip>
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <span className="inline-flex items-center gap-1">
                                  <Clock3Icon className="size-3" />
                                  {relativeTime(locale, t, node.profile_last_used_at)}
                                </span>
                              </TooltipTrigger>
                              <TooltipContent>
                                {formatTimestamp(locale, t, node.profile_last_used_at)}
                              </TooltipContent>
                            </Tooltip>
                          </div>
                        </div>
                      </div>
                    </button>
                  );
                })
              : null}
          </div>
          {activeItem ? (
            <div className="flex items-center justify-between border-t border-border/70 p-3">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() =>
                  setCandidateByIp((current) =>
                    new Map(current).set(activeItem.ip, new Set(defaultNodeIds(activeItem))),
                  )
                }
              >
                {t("Select all nodes")}
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() =>
                  setCandidateByIp((current) => new Map(current).set(activeItem.ip, new Set()))
                }
              >
                {t("Clear nodes")}
              </Button>
            </div>
          ) : null}
        </div>
      </div>
    </TooltipProvider>
  );
}
