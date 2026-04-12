import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useOutletContext } from "react-router-dom";
import { toast } from "sonner";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { formatApiErrorMessage } from "@/lib/error-messages";
import type { LoadSubscriptionResponse, ProxyScope } from "@/lib/types";
import { ProxiesPage } from "@/pages/ProxiesPage";
import type { RootOutletContext } from "@/routes/RootRoute";

export function ProxiesRoute() {
  const { t } = useI18n();
  const queryClient = useQueryClient();
  const { profiles, authMe, currentUser } = useOutletContext<RootOutletContext>();
  const [globalLoadResponse, setGlobalLoadResponse] = useState<LoadSubscriptionResponse | null>(
    null,
  );
  const canAccess =
    currentUser.status === "resolved" ? currentUser.identity.is_admin : Boolean(authMe?.is_admin);
  const accessDenied =
    currentUser.status === "anonymous" ||
    (currentUser.status === "resolved" && !currentUser.identity.is_admin);
  const authError = currentUser.status === "error" ? currentUser.message : null;

  const inventoryQuery = useQuery({
    queryKey: ["proxy-inventory"],
    queryFn: () => api.listProxyInventory({ scope: "all" }),
    enabled: canAccess,
  });
  const refreshProxyQueries = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["proxy-inventory"] }),
      queryClient.invalidateQueries({ queryKey: ["sessions"] }),
      queryClient.invalidateQueries({ queryKey: ["profiles"] }),
    ]);
  };

  const globalLoadMutation = useMutation({
    mutationFn: api.loadGlobalSubscription,
    onSuccess: async (response) => {
      setGlobalLoadResponse(response);
      toast.success(t("Imported {count} global proxies", { count: response.loaded_proxies }));
      await refreshProxyQueries();
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const reassignMutation = useMutation({
    mutationFn: ({ nodeId, scope }: { nodeId: string; scope: ProxyScope }) =>
      api.updateProxyAllocation(nodeId, { allocation_scope: scope }),
    onSuccess: async (_, variables) => {
      toast.success(t("Updated allocation for {nodeId}", { nodeId: variables.nodeId }));
      await refreshProxyQueries();
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  const deleteMutation = useMutation({
    mutationFn: (nodeId: string) => api.deleteProxyInventoryNode(nodeId),
    onSuccess: async (_, nodeId) => {
      toast.success(t("Deleted imported node {nodeId}", { nodeId }));
      await refreshProxyQueries();
    },
    onError: (error) => toast.error(formatApiErrorMessage(error, t)),
  });

  return (
    <ProxiesPage
      accessDenied={accessDenied}
      authError={authError}
      currentUser={currentUser}
      deletingNodeId={deleteMutation.isPending ? (deleteMutation.variables ?? null) : null}
      globalLoadError={
        globalLoadMutation.isError ? formatApiErrorMessage(globalLoadMutation.error, t) : null
      }
      globalLoadResponse={globalLoadResponse}
      inventory={inventoryQuery.data ?? null}
      inventoryError={
        inventoryQuery.isError ? formatApiErrorMessage(inventoryQuery.error, t) : null
      }
      inventoryLoading={inventoryQuery.isLoading}
      loadingGlobal={globalLoadMutation.isPending}
      onDeleteNode={async (nodeId) => {
        await deleteMutation.mutateAsync(nodeId);
      }}
      onLoadGlobal={async (payload) => {
        await globalLoadMutation.mutateAsync(payload);
      }}
      onReassignNode={async (nodeId, scope) => {
        await reassignMutation.mutateAsync({ nodeId, scope });
      }}
      profiles={profiles}
      reallocatingNodeId={
        reassignMutation.isPending ? (reassignMutation.variables?.nodeId ?? null) : null
      }
    />
  );
}
