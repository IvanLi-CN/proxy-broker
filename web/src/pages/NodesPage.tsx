import {
  ChevronDownIcon,
  DownloadIcon,
  Layers3Icon,
  PlayIcon,
  RefreshCwIcon,
  RouterIcon,
} from "lucide-react";
import { useState } from "react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { DataTablePanel } from "@/components/DataTablePanel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { NodesFiltersBar } from "@/features/nodes/components/NodesFiltersBar";
import { NodesTable } from "@/features/nodes/components/NodesTable";
import { useI18n } from "@/i18n";
import type { NodeFilterState } from "@/lib/nodes-view";
import type { NodeExportFormat, NodeListResponse, NodeViewMode } from "@/lib/types";

interface NodesPageProps {
  filterState: NodeFilterState;
  viewMode: NodeViewMode;
  bulkScope: "selected" | "all_filtered";
  data?: NodeListResponse | null;
  isLoading: boolean;
  isFetching?: boolean;
  isExporting?: boolean;
  isOpening?: boolean;
  error?: string | null;
  selectedIds: string[];
  onFilterChange: (patch: Partial<NodeFilterState>) => void;
  onResetFilters: () => void;
  onViewModeChange: (value: NodeViewMode) => void;
  onBulkScopeChange: (value: "selected" | "all_filtered") => void;
  onToggleSelect: (nodeId: string, checked: boolean) => void;
  onToggleSelectAll: (checked: boolean) => void;
  onClearSelection: () => void;
  onExport: (format: NodeExportFormat) => void | Promise<void>;
  onOpenSessions: () => void | Promise<void>;
}

const pageSizeOptions = [10, 25, 50, 100];

export function NodesPage({
  filterState,
  viewMode,
  bulkScope,
  data,
  isLoading,
  isFetching = false,
  isExporting = false,
  isOpening = false,
  error,
  selectedIds,
  onFilterChange,
  onResetFilters,
  onViewModeChange,
  onBulkScopeChange,
  onToggleSelect,
  onToggleSelectAll,
  onClearSelection,
  onExport,
  onOpenSessions,
}: NodesPageProps) {
  const { formatNumber, t } = useI18n();
  const [exportMenuOpen, setExportMenuOpen] = useState(false);
  const items = data?.items ?? [];
  const total = data?.total ?? 0;
  const currentPage = data?.page ?? filterState.page;
  const pageSize = data?.page_size ?? filterState.pageSize;
  const pageCount = Math.max(1, Math.ceil(total / Math.max(pageSize, 1)));
  const hasSelected = selectedIds.length > 0;
  const bulkDisabled = bulkScope === "selected" ? !hasSelected : total === 0;

  return (
    <div className="space-y-6">
      <header className="space-y-2">
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">{t("Nodes")}</h1>
        <p className="max-w-3xl text-sm leading-6 text-muted-foreground">
          {t(
            "Audit the current subscription snapshot by node, switch grouping modes, and launch bulk actions without leaving the table deck.",
          )}
        </p>
      </header>

      <NodesFiltersBar state={filterState} onChange={onFilterChange} onReset={onResetFilters} />

      {error ? (
        <ActionResponsePanel title={t("Nodes request failed")} description={error} tone="error" />
      ) : null}

      <DataTablePanel
        eyebrow={t("Node inventory")}
        title={t("Subscription nodes")}
        description={t(
          "Filters, sorting, and pagination come from the backend; the current page can then be regrouped locally by IP, region, or subscription source.",
        )}
        chips={[
          t(total === 1 ? "{count} node" : "{count} nodes", { count: formatNumber(total) }),
          t(selectedIds.length === 1 ? "{count} selected" : "{count} selected", {
            count: formatNumber(selectedIds.length),
          }),
          isFetching ? t("refreshing") : t("snapshot ready"),
        ]}
        actions={
          <Badge
            variant="outline"
            className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
          >
            <RefreshCwIcon className="mr-1 size-3.5" />
            {isFetching ? t("syncing") : t("stable")}
          </Badge>
        }
      >
        <div className="space-y-5">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <Tabs
              value={viewMode}
              onValueChange={(value) => onViewModeChange(value as NodeViewMode)}
            >
              <TabsList className="grid grid-cols-2 rounded-2xl border border-border/70 bg-card/80 p-1 sm:grid-cols-4">
                <TabsTrigger value="flat">{t("Flat")}</TabsTrigger>
                <TabsTrigger value="group_by_ip">{t("By IP")}</TabsTrigger>
                <TabsTrigger value="group_by_region">{t("By region")}</TabsTrigger>
                <TabsTrigger value="group_by_subscription">{t("By subscription")}</TabsTrigger>
              </TabsList>
            </Tabs>

            <div className="flex flex-wrap items-center gap-2">
              <Select
                value={bulkScope}
                onValueChange={(value) => onBulkScopeChange(value as "selected" | "all_filtered")}
              >
                <SelectTrigger className="min-w-[180px] bg-background/80">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="selected">{t("Selected nodes")}</SelectItem>
                  <SelectItem value="all_filtered">{t("All filtered nodes")}</SelectItem>
                </SelectContent>
              </Select>
              <Button variant="outline" onClick={onClearSelection} disabled={!hasSelected}>
                {t("Clear selection")}
              </Button>
              <Popover open={exportMenuOpen} onOpenChange={setExportMenuOpen}>
                <PopoverTrigger asChild>
                  <Button variant="outline" disabled={bulkDisabled || isExporting}>
                    <DownloadIcon className="size-4" />
                    {isExporting ? t("Exporting...") : t("Export")}
                    <ChevronDownIcon className="size-4 text-muted-foreground" />
                  </Button>
                </PopoverTrigger>
                <PopoverContent align="end" className="w-64 p-2">
                  <div className="px-2 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
                    {t("Export format")}
                  </div>
                  <div className="mt-1 space-y-1">
                    <Button
                      variant="ghost"
                      className="w-full justify-start"
                      onClick={() => {
                        setExportMenuOpen(false);
                        void onExport("link_lines");
                      }}
                    >
                      {t("Node links (.txt, one per line)")}
                    </Button>
                    <Button
                      variant="ghost"
                      className="w-full justify-start"
                      onClick={() => {
                        setExportMenuOpen(false);
                        void onExport("csv");
                      }}
                    >
                      {t("CSV metadata (.csv)")}
                    </Button>
                  </div>
                </PopoverContent>
              </Popover>
              <Button onClick={() => void onOpenSessions()} disabled={bulkDisabled || isOpening}>
                <PlayIcon className="size-4" />
                {isOpening ? t("Creating sessions...") : t("Create sessions")}
              </Button>
            </div>
          </div>

          <div className="flex flex-wrap items-center justify-between gap-3 rounded-[24px] border border-border/70 bg-background/80 px-4 py-3">
            <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
              <Layers3Icon className="size-4 text-primary" />
              {t("Bulk scope")}
              <span className="font-medium text-foreground">
                {bulkScope === "selected" ? t("Selected nodes") : t("All filtered nodes")}
              </span>
            </div>
            <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
              <RouterIcon className="size-4 text-primary" />
              {t("Page {page} of {count}", {
                page: formatNumber(currentPage),
                count: formatNumber(pageCount),
              })}
            </div>
          </div>

          <NodesTable
            items={items}
            isLoading={isLoading}
            viewMode={viewMode}
            selectedIds={selectedIds}
            onToggleSelect={onToggleSelect}
            onToggleSelectAll={onToggleSelectAll}
          />

          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex flex-wrap items-center gap-2">
              <Button
                variant="outline"
                onClick={() => onFilterChange({ page: Math.max(1, currentPage - 1) })}
                disabled={currentPage <= 1}
              >
                {t("Previous")}
              </Button>
              <Button
                variant="outline"
                onClick={() => onFilterChange({ page: Math.min(pageCount, currentPage + 1) })}
                disabled={currentPage >= pageCount}
              >
                {t("Next")}
              </Button>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-sm text-muted-foreground">{t("Rows per page")}</span>
              <Select
                value={String(pageSize)}
                onValueChange={(value) => onFilterChange({ pageSize: Number(value), page: 1 })}
              >
                <SelectTrigger className="w-[112px] bg-background/80">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {pageSizeOptions.map((option) => (
                    <SelectItem key={option} value={String(option)}>
                      {option}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>
      </DataTablePanel>
    </div>
  );
}
