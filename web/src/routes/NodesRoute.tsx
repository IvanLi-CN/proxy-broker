import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import {
  buildNodeExportRequest,
  buildNodeListQuery,
  buildNodeOpenSessionsRequest,
  defaultNodeFilterState,
  type NodeFilterState,
} from "@/lib/nodes-view";
import type { NodeExportFormat, NodeViewMode } from "@/lib/types";
import { NodesPage } from "@/pages/NodesPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function NodesRoute() {
  const { t } = useI18n();
  const { profileId } = useOutletContext<RootOutletContext>();
  const queryClient = useQueryClient();
  const previousProfileId = useRef(profileId);
  const [filterState, setFilterState] = useState<NodeFilterState>(defaultNodeFilterState);
  const [viewMode, setViewMode] = useState<NodeViewMode>("flat");
  const [bulkScope, setBulkScope] = useState<"selected" | "all_filtered">("selected");
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const query = useMemo(() => buildNodeListQuery(filterState), [filterState]);

  const nodesQuery = useQuery({
    queryKey: ["nodes", profileId, query],
    queryFn: () => api.queryNodes(profileId, query),
    refetchInterval: 10_000,
  });

  const exportMutation = useMutation({
    mutationFn: async (format: NodeExportFormat) => {
      const body = await api.exportNodes(
        profileId,
        buildNodeExportRequest(bulkScope, selectedIds, query, format),
      );
      const isCsv = format === "csv";
      const blob = new Blob([body], {
        type: isCsv ? "text/csv;charset=utf-8" : "text/plain;charset=utf-8",
      });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = isCsv
        ? `proxy-broker-nodes-${profileId}.csv`
        : `proxy-broker-node-links-${profileId}.txt`;
      link.click();
      URL.revokeObjectURL(url);
      return format;
    },
    onSuccess: (format) =>
      toast.success(
        format === "csv" ? t("Exported node inventory as CSV") : t("Exported node links as TXT"),
      ),
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const openMutation = useMutation({
    mutationFn: () =>
      api.openNodeSessions(profileId, buildNodeOpenSessionsRequest(bulkScope, selectedIds, query)),
    onSuccess: async (data) => {
      const opened = data.sessions.length;
      const failed = data.failures.length;
      toast.success(
        failed > 0
          ? t("Created {opened} sessions with {failed} failures", { opened, failed })
          : t("Created {count} sessions", { count: opened }),
      );
      if (failed > 0) {
        const firstFailure = data.failures[0];
        toast.error(`${firstFailure?.code}: ${firstFailure?.message}`);
      }
      await queryClient.invalidateQueries({ queryKey: ["nodes", profileId] });
      await queryClient.invalidateQueries({ queryKey: ["sessions", profileId] });
      await queryClient.invalidateQueries({ queryKey: ["suggested-port", profileId] });
      setSelectedIds([]);
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  useEffect(() => {
    if (previousProfileId.current === profileId) {
      return;
    }
    previousProfileId.current = profileId;
    setFilterState(defaultNodeFilterState);
    setViewMode("flat");
    setBulkScope("selected");
    setSelectedIds([]);
  }, [profileId]);

  return (
    <NodesPage
      bulkScope={bulkScope}
      data={nodesQuery.data ?? null}
      error={nodesQuery.isError ? formatApiErrorMessage(nodesQuery.error, t) : null}
      filterState={filterState}
      isExporting={exportMutation.isPending}
      isFetching={nodesQuery.isFetching}
      isLoading={nodesQuery.isLoading}
      isOpening={openMutation.isPending}
      onBulkScopeChange={setBulkScope}
      onClearSelection={() => setSelectedIds([])}
      onExport={async (format) => {
        await exportMutation.mutateAsync(format);
      }}
      onFilterChange={(patch) => {
        setFilterState((current) => ({ ...current, ...patch }));
      }}
      onOpenSessions={async () => {
        await openMutation.mutateAsync();
      }}
      onResetFilters={() => setFilterState(defaultNodeFilterState)}
      onToggleSelect={(nodeId, checked) => {
        setSelectedIds((current) =>
          checked
            ? Array.from(new Set([...current, nodeId]))
            : current.filter((item) => item !== nodeId),
        );
      }}
      onToggleSelectAll={(checked) => {
        setSelectedIds((current) => {
          const pageIds = (nodesQuery.data?.items ?? []).map((item) => item.node_id);
          if (checked) {
            return Array.from(new Set([...current, ...pageIds]));
          }
          return current.filter((item) => !pageIds.includes(item));
        });
      }}
      onViewModeChange={setViewMode}
      selectedIds={selectedIds}
      viewMode={viewMode}
    />
  );
}
