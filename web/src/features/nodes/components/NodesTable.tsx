import { DatabaseZapIcon, LoaderCircleIcon } from "lucide-react";

import { EmptyPanel } from "@/components/EmptyPanel";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useI18n } from "@/i18n";
import { formatCountryName, formatGeoLabel, formatLatency, formatTimestamp } from "@/lib/format";
import { areAllPageNodesSelected, groupNodeItems } from "@/lib/nodes-view";
import type { NodeListItem, NodeViewMode } from "@/lib/types";
import { cn } from "@/lib/utils";

interface NodesTableProps {
  items: NodeListItem[];
  isLoading?: boolean;
  viewMode: NodeViewMode;
  selectedIds: string[];
  onToggleSelect: (nodeId: string, checked: boolean) => void;
  onToggleSelectAll: (checked: boolean) => void;
}

export function NodesTable({
  items,
  isLoading,
  viewMode,
  selectedIds,
  onToggleSelect,
  onToggleSelectAll,
}: NodesTableProps) {
  const { locale, t } = useI18n();

  if (isLoading && items.length === 0) {
    return (
      <EmptyPanel
        title={t("Loading nodes")}
        description={t("Querying the current subscription snapshot for node inventory.")}
        icon={LoaderCircleIcon}
        hint={t("The current page will populate after the first nodes response lands.")}
      />
    );
  }

  if (items.length === 0) {
    return (
      <EmptyPanel
        title={t("No nodes found")}
        description={t("Adjust the filters or load a subscription to populate this workspace.")}
        icon={DatabaseZapIcon}
      />
    );
  }

  const groups = groupNodeItems(items, viewMode);
  const allSelected = areAllPageNodesSelected(items, selectedIds);

  return (
    <ScrollArea className="rounded-[28px] border border-border/70 bg-card/90 shadow-sm">
      <Table className="min-w-[1200px]">
        <TableHeader>
          <TableRow className="border-b border-border/70 bg-muted/20">
            <TableHead className="w-12 px-4">
              <Checkbox
                checked={allSelected}
                onCheckedChange={(checked) => onToggleSelectAll(checked === true)}
              />
            </TableHead>
            <TableHead>{t("Node")}</TableHead>
            <TableHead>{t("IPs")}</TableHead>
            <TableHead>{t("Geo")}</TableHead>
            <TableHead>{t("Probe")}</TableHead>
            <TableHead>{t("Latency")}</TableHead>
            <TableHead>{t("Last used")}</TableHead>
            <TableHead className="pr-4 text-right">{t("Sessions")}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {groups.map((group) => (
            <GroupRows
              key={group.key}
              groupLabel={group.label}
              hideLabel={viewMode === "flat"}
              items={group.items}
              selectedIds={selectedIds}
              onToggleSelect={onToggleSelect}
              locale={locale}
              t={t}
            />
          ))}
        </TableBody>
      </Table>
      <ScrollBar orientation="horizontal" />
    </ScrollArea>
  );
}

function GroupRows({
  groupLabel,
  hideLabel,
  items,
  selectedIds,
  onToggleSelect,
  locale,
  t,
}: {
  groupLabel: string;
  hideLabel: boolean;
  items: NodeListItem[];
  selectedIds: string[];
  onToggleSelect: (nodeId: string, checked: boolean) => void;
  locale: ReturnType<typeof useI18n>["locale"];
  t: ReturnType<typeof useI18n>["t"];
}) {
  return (
    <>
      {!hideLabel ? (
        <TableRow className="border-y border-border/60 bg-background/80">
          <TableCell colSpan={8} className="px-4 py-2">
            <div className="text-xs font-semibold uppercase tracking-[0.22em] text-muted-foreground">
              {groupLabel}
            </div>
          </TableCell>
        </TableRow>
      ) : null}
      {items.map((item) => {
        const selected = selectedIds.includes(item.node_id);
        return (
          <TableRow
            key={item.node_id}
            className={cn("[&_td]:py-3", selected && "bg-primary/[0.04]")}
          >
            <TableCell className="px-4">
              <Checkbox
                checked={selected}
                onCheckedChange={(checked) => onToggleSelect(item.node_id, checked === true)}
              />
            </TableCell>
            <TableCell>
              <div className="space-y-1">
                <div className="font-medium">{item.proxy_name}</div>
                <div className="text-xs text-muted-foreground">{item.server}</div>
                <Badge
                  variant="outline"
                  className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.14em]"
                >
                  {item.proxy_type}
                </Badge>
              </div>
            </TableCell>
            <TableCell>
              <div className="space-y-1 text-xs md:text-sm">
                <div className="font-mono">{item.preferred_ip ?? t("No preferred IP")}</div>
                <div className="text-muted-foreground">
                  {t("v4: {value}", { value: item.ipv4 ?? "—" })}
                </div>
                <div className="text-muted-foreground">
                  {t("v6: {value}", { value: item.ipv6 ?? "—" })}
                </div>
              </div>
            </TableCell>
            <TableCell>
              <div className="space-y-1">
                <div className="font-medium">
                  {formatCountryName(locale, item.country_code, item.country_name) ?? t("Unknown")}
                </div>
                <div className="text-xs text-muted-foreground">
                  {[formatGeoLabel(locale, item.region_name), formatGeoLabel(locale, item.city)]
                    .filter(Boolean)
                    .join(" / ") || t("No city metadata")}
                </div>
              </div>
            </TableCell>
            <TableCell>
              <Badge
                variant="outline"
                className={cn(
                  "rounded-full px-3 py-1 text-[11px] uppercase tracking-[0.14em]",
                  item.probe_status === "reachable"
                    ? "border-emerald-500/20 bg-emerald-500/[0.08] text-emerald-700 dark:text-emerald-300"
                    : item.probe_status === "unreachable"
                      ? "border-amber-500/25 bg-amber-500/[0.1] text-amber-700 dark:text-amber-300"
                      : "border-border/70 bg-background/80 text-muted-foreground",
                )}
              >
                {item.probe_status === "reachable"
                  ? t("Reachable")
                  : item.probe_status === "unreachable"
                    ? t("Unreachable")
                    : t("Unprobed")}
              </Badge>
            </TableCell>
            <TableCell className="font-mono text-xs md:text-sm">
              {formatLatency(locale, t, item.best_latency_ms)}
            </TableCell>
            <TableCell className="text-xs md:text-sm">
              {formatTimestamp(locale, t, item.last_used_at)}
            </TableCell>
            <TableCell className="pr-4 text-right">
              <span className="font-mono text-sm">{item.session_count}</span>
            </TableCell>
          </TableRow>
        );
      })}
    </>
  );
}
