import {
  ActivityIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  Layers3Icon,
  PlusIcon,
  RefreshCcwIcon,
  Trash2Icon,
} from "lucide-react";
import { Fragment, useEffect, useState } from "react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { DataTablePanel } from "@/components/DataTablePanel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ProfileProxyPolicyCard } from "@/features/proxies/components/ProfileProxyPolicyCard";
import { ProxyLoadCard } from "@/features/proxies/components/ProxyLoadCard";
import type { ProxyNodeLiveState } from "@/hooks/use-proxy-operation-events";
import { useI18n } from "@/i18n";
import {
  formatCountryName,
  formatLatency,
  formatSubscriptionExpire,
  formatSubscriptionQuota,
  formatTimestamp,
  optionalNumber,
} from "@/lib/format";
import type {
  CurrentUserState,
  ListProxyImportResponse,
  LoadSubscriptionRequest,
  LoadSubscriptionResponse,
  OpenBatchByNodeRequest,
  OpenSessionByNodeRequest,
  ProfileProxySettings,
  ProxyCatalogGroupItem,
  ProxyCatalogNodeItem,
  ProxyCatalogResponse,
  ProxyImportItem,
  ProxyImportKind,
  ProxyScope,
} from "@/lib/types";

function encodeScope(scope: ProxyScope) {
  return scope.type === "global" ? "global" : `profile:${scope.profile_id}`;
}

function decodeScope(value: string): ProxyScope {
  if (value === "global") {
    return { type: "global" };
  }
  return { type: "profile", profile_id: value.slice("profile:".length) };
}

function formatScopeLabel(scope: ProxyScope, t: ReturnType<typeof useI18n>["t"]) {
  return scope.type === "global"
    ? t("Global pool")
    : t("Profile {profileId}", { profileId: scope.profile_id });
}

function formatImportKind(kind: ProxyImportKind, t: ReturnType<typeof useI18n>["t"]) {
  return kind === "subscription" ? t("Subscription import") : t("Node group import");
}

function formatImportLabel(item: ProxyImportItem) {
  return item.name?.trim() || item.import_id;
}

function formatImportSourceTitle(item: ProxyImportItem) {
  const sourceTitle = item.subscription_metadata?.source_title?.trim();
  const displayName = item.name?.trim();
  if (!sourceTitle || sourceTitle === displayName) {
    return null;
  }
  return sourceTitle;
}

function PageHeader({
  scopeBadge,
  title,
  description,
}: {
  scopeBadge: string;
  title: string;
  description: string;
}) {
  return (
    <div className="space-y-2">
      <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
        {scopeBadge}
      </div>
      <div className="space-y-1">
        <h1 className="text-3xl font-semibold tracking-tight text-foreground">{title}</h1>
        <p className="max-w-3xl text-sm leading-6 text-muted-foreground">{description}</p>
      </div>
    </div>
  );
}

function InventoryProfiles({ effectiveProfileIds }: { effectiveProfileIds: string[] }) {
  const { formatNumber, t } = useI18n();
  if (effectiveProfileIds.length === 0) {
    return <span className="text-xs text-muted-foreground">{t("No active profiles")}</span>;
  }

  return (
    <div className="flex flex-wrap gap-1">
      {effectiveProfileIds.slice(0, 3).map((profileId) => (
        <Badge
          key={profileId}
          variant="secondary"
          className="rounded-full bg-muted/70 px-1.5 py-0 text-[10px]"
        >
          {profileId}
        </Badge>
      ))}
      {effectiveProfileIds.length > 3 ? (
        <Badge variant="outline" className="rounded-full px-1.5 py-0 text-[10px]">
          {t("+{count} more", { count: formatNumber(effectiveProfileIds.length - 3) })}
        </Badge>
      ) : null}
    </div>
  );
}

function NodeStatusCell({
  node,
  liveState,
}: {
  node: ProxyCatalogNodeItem;
  liveState?: ProxyNodeLiveState;
}) {
  const { locale, t } = useI18n();
  const metadata =
    node.ip_metadata.find((record) => record.ip === node.primary_ip) ?? node.ip_metadata[0] ?? null;
  const country = formatCountryName(locale, metadata?.country_code, metadata?.country_name);
  const city = metadata?.city ?? metadata?.region_name ?? null;
  const successCount = metadata?.last_probe_samples.filter(
    (value) => typeof value === "number",
  ).length;

  return (
    <div className="space-y-1 text-xs leading-5 text-muted-foreground">
      <div>
        {country || city ? [country, city].filter(Boolean).join(" / ") : t("No geo metadata yet")}
      </div>
      {liveState ? (
        <div className="font-medium text-primary">
          {liveState.kind === "proxy_latency_probe"
            ? t("Round {round}/{total}: {latency}", {
                round: liveState.latestRound ?? 0,
                total: liveState.samplesTotal ?? 5,
                latency:
                  liveState.latestSampleMs == null
                    ? t("timeout")
                    : formatLatency(locale, t, liveState.latestSampleMs),
              })
            : (liveState.message ?? t("Refreshing metadata"))}
        </div>
      ) : metadata ? (
        <>
          <div>
            {metadata.median_latency_ms != null
              ? t("Median {latency}", {
                  latency: formatLatency(locale, t, metadata.median_latency_ms),
                })
              : successCount === 0 && metadata.last_probe_samples.length > 0
                ? t("Probe failed (0/5)")
                : t("No probe median yet")}
          </div>
          <div>{formatTimestamp(locale, t, metadata.updated_at)}</div>
        </>
      ) : (
        <div>{t("No probe data yet")}</div>
      )}
    </div>
  );
}

export function NodePinnedSessionDialog({
  open,
  node,
  suggestedPort,
  isPending,
  onOpenChange,
  onSubmit,
}: {
  open: boolean;
  node: ProxyCatalogNodeItem | null;
  suggestedPort?: number | null;
  isPending: boolean;
  onOpenChange: (nextOpen: boolean) => void;
  onSubmit: (payload: OpenSessionByNodeRequest) => void | Promise<void>;
}) {
  const { t } = useI18n();
  const [desiredPort, setDesiredPort] = useState("");

  useEffect(() => {
    if (open) {
      setDesiredPort("");
    }
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{t("Create node-pinned session")}</DialogTitle>
          <DialogDescription>
            {node
              ? t(
                  "Open one listener pinned to {proxyName}. The backend will keep the node binding fixed and use the primary resolved IP.",
                  { proxyName: node.proxy_name },
                )
              : t("Pick one node before opening the create-session form.")}
          </DialogDescription>
        </DialogHeader>

        {node ? (
          <form
            className="space-y-4"
            onSubmit={async (event) => {
              event.preventDefault();
              await onSubmit({
                node_id: node.node_id,
                desired_port: optionalNumber(desiredPort),
              });
            }}
          >
            <div className="rounded-xl border border-border/70 bg-muted/20 p-3 text-xs text-muted-foreground">
              <div className="font-medium text-foreground">{node.proxy_name}</div>
              <div className="mt-1 font-mono">{node.server}</div>
              <div className="mt-1">
                {t("Primary IP: {ip}", {
                  ip: node.primary_ip ?? node.resolved_ips[0] ?? t("No resolved IPs"),
                })}
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="node-session-desired-port">{t("Desired port (optional)")}</Label>
              <Input
                id="node-session-desired-port"
                inputMode="numeric"
                pattern="[0-9]*"
                placeholder={suggestedPort?.toString() ?? "10080"}
                value={desiredPort}
                onChange={(event) => setDesiredPort(event.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                {t(
                  "Leave this blank to auto-assign the next available port. Set it when you need a predictable listener port.",
                )}
              </p>
            </div>

            <DialogFooter>
              <Button type="submit" disabled={isPending}>
                <PlusIcon className="size-3.5" />
                {isPending ? t("Creating session...") : t("Create session")}
              </Button>
            </DialogFooter>
          </form>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

export function NodePinnedBatchDialog({
  open,
  nodes,
  suggestedPort,
  isPending,
  onOpenChange,
  onSubmit,
}: {
  open: boolean;
  nodes: ProxyCatalogNodeItem[];
  suggestedPort?: number | null;
  isPending: boolean;
  onOpenChange: (nextOpen: boolean) => void;
  onSubmit: (payload: OpenBatchByNodeRequest) => void | Promise<void>;
}) {
  const { t } = useI18n();
  const [desiredPorts, setDesiredPorts] = useState<Record<string, string>>({});

  useEffect(() => {
    if (open) {
      setDesiredPorts(
        Object.fromEntries(nodes.map((node) => [node.node_id, ""])) as Record<string, string>,
      );
    }
  }, [nodes, open]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("Create node-pinned sessions")}</DialogTitle>
          <DialogDescription>
            {t(
              "Review the selected nodes before opening the batch. Each row may keep auto-assigned ports or request an explicit one.",
            )}
          </DialogDescription>
        </DialogHeader>

        <form
          className="space-y-4"
          onSubmit={async (event) => {
            event.preventDefault();
            await onSubmit({
              requests: nodes.map((node) => ({
                node_id: node.node_id,
                desired_port: optionalNumber(desiredPorts[node.node_id] ?? ""),
              })),
            });
          }}
        >
          <div className="space-y-3">
            {nodes.map((node, index) => (
              <div
                key={node.node_id}
                className="grid gap-3 rounded-xl border border-border/70 bg-muted/10 p-3 md:grid-cols-[minmax(0,1fr)_180px]"
              >
                <div className="space-y-1 text-xs text-muted-foreground">
                  <div className="font-medium text-foreground">{node.proxy_name}</div>
                  <div className="font-mono">{node.server}</div>
                  <div>
                    {t("Primary IP: {ip}", {
                      ip: node.primary_ip ?? node.resolved_ips[0] ?? t("No resolved IPs"),
                    })}
                  </div>
                </div>
                <div className="space-y-2">
                  <Label htmlFor={`node-batch-port-${node.node_id}`}>
                    {t("Desired port (optional)")}
                  </Label>
                  <Input
                    id={`node-batch-port-${node.node_id}`}
                    inputMode="numeric"
                    pattern="[0-9]*"
                    placeholder={
                      suggestedPort != null ? String(suggestedPort + index) : String(10080 + index)
                    }
                    value={desiredPorts[node.node_id] ?? ""}
                    onChange={(event) =>
                      setDesiredPorts((current) => ({
                        ...current,
                        [node.node_id]: event.target.value,
                      }))
                    }
                  />
                </div>
              </div>
            ))}
          </div>

          <DialogFooter>
            <Button type="submit" disabled={isPending || nodes.length === 0}>
              <PlusIcon className="size-3.5" />
              {isPending ? t("Creating sessions...") : t("Create sessions")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

export function DeleteImportConfirmDialog({
  open,
  item,
  isPending,
  onOpenChange,
  onConfirm,
}: {
  open: boolean;
  item: ProxyImportItem | null;
  isPending: boolean;
  onOpenChange: (nextOpen: boolean) => void;
  onConfirm: (importId: string) => void | Promise<void>;
}) {
  const { t } = useI18n();
  const importLabel = item ? formatImportLabel(item) : null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t("Confirm deletion")}</DialogTitle>
          <DialogDescription>
            {item
              ? t(
                  "Delete imported source {name}? This removes the whole grouped import and its child nodes from the current scope.",
                  { name: importLabel ?? item.import_id },
                )
              : t("Pick one imported source before confirming deletion.")}
          </DialogDescription>
        </DialogHeader>

        {item ? (
          <>
            <div className="rounded-xl border border-destructive/20 bg-destructive/5 p-3 text-xs text-muted-foreground">
              <div className="font-medium text-foreground">{importLabel}</div>
              <div className="mt-1">{formatImportKind(item.import_kind, t)}</div>
              <div className="mt-1">{formatScopeLabel(item.source_scope, t)}</div>
            </div>

            <DialogFooter>
              <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isPending}>
                {t("Cancel")}
              </Button>
              <Button
                variant="destructive"
                onClick={() => void onConfirm(item.import_id)}
                disabled={isPending}
              >
                <Trash2Icon className="size-3.5" />
                {isPending ? t("Deleting...") : t("Delete")}
              </Button>
            </DialogFooter>
          </>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

interface SharedCatalogProps {
  proxyCatalog?: ProxyCatalogResponse | null;
  proxyCatalogLoading: boolean;
  proxyCatalogError?: string | null;
  liveConnectionState: string;
  liveNodeStates: Record<string, ProxyNodeLiveState>;
  queueingOperation: boolean;
  onRefreshNodes: (nodeIds: string[]) => void | Promise<void>;
  onProbeNodes: (nodeIds: string[]) => void | Promise<void>;
}

interface GlobalProxiesPageProps extends SharedCatalogProps {
  mode: "global";
  profiles: string[];
  currentUser: CurrentUserState;
  accessDenied?: boolean;
  authError?: string | null;
  globalLoadResponse?: LoadSubscriptionResponse | null;
  globalLoadError?: string | null;
  loadingGlobal: boolean;
  proxyImports?: ListProxyImportResponse | null;
  proxyImportsLoading: boolean;
  proxyImportsError?: string | null;
  reallocatingImportId?: string | null;
  deletingImportId?: string | null;
  onLoadGlobal: (payload: LoadSubscriptionRequest) => void | Promise<void>;
  onReassignImport: (importId: string, scope: ProxyScope) => void | Promise<void>;
  onDeleteImport: (importId: string) => void | Promise<void>;
}

interface ProfileProxiesPageProps extends SharedCatalogProps {
  mode: "profile";
  profileId: string;
  currentUser: CurrentUserState;
  suggestedPort?: number | null;
  profileLoadResponse?: LoadSubscriptionResponse | null;
  profileLoadError?: string | null;
  loadingProfile: boolean;
  proxySettings?: ProfileProxySettings | null;
  proxySettingsLoading?: boolean;
  proxySettingsError?: string | null;
  updatingSettings?: boolean;
  showProxyPolicy?: boolean;
  onLoadProfile: (payload: LoadSubscriptionRequest) => void | Promise<void>;
  onToggleUseGlobalProxies: (nextValue: boolean) => void | Promise<void>;
  onDeleteImport?: (importId: string) => void | Promise<void>;
  onOpenSessionByNode: (payload: OpenSessionByNodeRequest) => void | Promise<void>;
  onOpenBatchByNode: (payload: OpenBatchByNodeRequest) => void | Promise<void>;
  deletingImportId?: string | null;
  openingSessionNodeId?: string | null;
  openingBatch?: boolean;
}

export type ProxiesPageProps = GlobalProxiesPageProps | ProfileProxiesPageProps;

export function ProxiesPage(props: ProxiesPageProps) {
  if (props.mode === "global") {
    return <GlobalProxiesView {...props} />;
  }

  return <ProfileProxiesView {...props} />;
}

function GlobalProxiesView({
  profiles,
  accessDenied = false,
  authError = null,
  globalLoadResponse,
  globalLoadError,
  loadingGlobal,
  proxyImportsError,
  reallocatingImportId = null,
  deletingImportId = null,
  onLoadGlobal,
  onReassignImport,
  onDeleteImport,
  proxyCatalog,
  proxyCatalogLoading,
  proxyCatalogError,
  liveConnectionState,
  liveNodeStates,
  queueingOperation,
  onRefreshNodes,
  onProbeNodes,
}: GlobalProxiesPageProps) {
  const { t } = useI18n();

  if (authError) {
    return (
      <div className="space-y-5">
        <PageHeader
          scopeBadge={t("Global")}
          title={t("Proxy")}
          description={t("Manage the shared global pool and every profile allocation from here.")}
        />
        <ActionResponsePanel
          title={t("Current user unavailable")}
          description={authError}
          tone="error"
        />
      </div>
    );
  }

  if (accessDenied) {
    return (
      <div className="space-y-5">
        <PageHeader
          scopeBadge={t("Global")}
          title={t("Proxy")}
          description={t("Manage the shared global pool and every profile allocation from here.")}
        />
        <ActionResponsePanel
          title={t("Admin access required")}
          description={t(
            "The global config can change the shared pool and profile allocations, so only admins can open it.",
          )}
          tone="error"
        />
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <PageHeader
        scopeBadge={t("Global")}
        title={t("Proxy")}
        description={t("Manage the shared global pool and every profile allocation from here.")}
      />

      <ProxyLoadCard
        defaultValue="https://example.com/global-subscription.yaml"
        description={t(
          "Import one subscription source or one node group into the shared pool. Profiles that keep global usage enabled inherit these nodes immediately.",
        )}
        error={globalLoadError}
        eyebrow={t("Global pool")}
        onSubmit={onLoadGlobal}
        pending={loadingGlobal}
        response={globalLoadResponse}
        scopeChip={t("allocation defaults to global")}
        submitLabel={t("Import global pool")}
        successDescription={t(
          "Imported {proxyCount} proxies across {ipCount} distinct IPs into the global pool.",
          {
            proxyCount: globalLoadResponse?.loaded_proxies ?? 0,
            ipCount: globalLoadResponse?.distinct_ips ?? 0,
          },
        )}
        successTitle={t("Global pool updated")}
        title={t("Import global proxy pool")}
      />

      {proxyImportsError ? (
        <ActionResponsePanel
          title={t("Proxy imports unavailable")}
          description={proxyImportsError}
          tone="error"
        />
      ) : null}

      <GroupedProxyCatalogPanel
        mode="global"
        profiles={profiles}
        proxyCatalog={proxyCatalog}
        proxyCatalogLoading={proxyCatalogLoading}
        proxyCatalogError={proxyCatalogError}
        liveConnectionState={liveConnectionState}
        liveNodeStates={liveNodeStates}
        queueingOperation={queueingOperation}
        onRefreshNodes={onRefreshNodes}
        onProbeNodes={onProbeNodes}
        reallocatingImportId={reallocatingImportId}
        deletingImportId={deletingImportId}
        onReassignImport={onReassignImport}
        onDeleteImport={onDeleteImport}
      />
    </div>
  );
}

function ProfileProxiesView({
  profileId,
  suggestedPort,
  profileLoadResponse,
  profileLoadError,
  loadingProfile,
  proxySettings,
  proxySettingsLoading = false,
  proxySettingsError,
  updatingSettings = false,
  showProxyPolicy = true,
  onLoadProfile,
  onToggleUseGlobalProxies,
  onDeleteImport,
  proxyCatalog,
  proxyCatalogLoading,
  proxyCatalogError,
  liveConnectionState,
  liveNodeStates,
  queueingOperation,
  onRefreshNodes,
  onProbeNodes,
  onOpenSessionByNode,
  onOpenBatchByNode,
  deletingImportId = null,
  openingSessionNodeId = null,
  openingBatch = false,
}: ProfileProxiesPageProps) {
  const { t } = useI18n();

  return (
    <div className="space-y-5">
      <PageHeader
        scopeBadge={t("Current config")}
        title={t("Proxy")}
        description={t(
          "Manage local imports and whether {profileId} also composes the global pool.",
          {
            profileId,
          },
        )}
      />

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1.45fr)_minmax(320px,0.9fr)]">
        <ProxyLoadCard
          defaultValue="https://example.com/profile-subscription.yaml"
          description={t(
            "Import one subscription source or one node group for this profile only. They stay local unless you later reassign them from the global config.",
          )}
          error={profileLoadError}
          eyebrow={t("Current config")}
          onSubmit={onLoadProfile}
          pending={loadingProfile}
          response={profileLoadResponse}
          scopeChip={t("Scoped to {profileId} only.", { profileId })}
          submitLabel={t("Import local pool")}
          successDescription={t(
            "Imported {proxyCount} proxies across {ipCount} distinct IPs into profile {profileId}.",
            {
              proxyCount: profileLoadResponse?.loaded_proxies ?? 0,
              ipCount: profileLoadResponse?.distinct_ips ?? 0,
              profileId,
            },
          )}
          successTitle={t("Local pool updated")}
          title={t("Import local proxy pool")}
        />
        {showProxyPolicy ? (
          <ProfileProxyPolicyCard
            profileId={profileId}
            useGlobalProxies={proxySettings?.use_global_proxies ?? true}
            proxySettingsLoading={proxySettingsLoading}
            updatingSettings={updatingSettings}
            proxySettingsError={proxySettingsError}
            onToggleUseGlobalProxies={onToggleUseGlobalProxies}
          />
        ) : null}
      </div>

      <GroupedProxyCatalogPanel
        mode="profile"
        proxyCatalog={proxyCatalog}
        proxyCatalogLoading={proxyCatalogLoading}
        proxyCatalogError={proxyCatalogError}
        liveConnectionState={liveConnectionState}
        liveNodeStates={liveNodeStates}
        queueingOperation={queueingOperation}
        onRefreshNodes={onRefreshNodes}
        onProbeNodes={onProbeNodes}
        currentProfileId={profileId}
        deletingImportId={deletingImportId}
        onDeleteImport={onDeleteImport}
        suggestedPort={suggestedPort}
        onOpenSessionByNode={onOpenSessionByNode}
        onOpenBatchByNode={onOpenBatchByNode}
        openingSessionNodeId={openingSessionNodeId}
        openingBatch={openingBatch}
      />
    </div>
  );
}

function GroupedProxyCatalogPanel({
  mode,
  profiles = [],
  currentProfileId,
  proxyCatalog,
  proxyCatalogLoading,
  proxyCatalogError,
  liveConnectionState,
  liveNodeStates,
  queueingOperation,
  onRefreshNodes,
  onProbeNodes,
  onReassignImport,
  onDeleteImport,
  suggestedPort,
  reallocatingImportId = null,
  deletingImportId = null,
  onOpenSessionByNode,
  onOpenBatchByNode,
  openingSessionNodeId = null,
  openingBatch = false,
}: {
  mode: "global" | "profile";
  profiles?: string[];
  currentProfileId?: string;
  proxyCatalog?: ProxyCatalogResponse | null;
  proxyCatalogLoading: boolean;
  proxyCatalogError?: string | null;
  liveConnectionState: string;
  liveNodeStates: Record<string, ProxyNodeLiveState>;
  queueingOperation: boolean;
  onRefreshNodes: (nodeIds: string[]) => void | Promise<void>;
  onProbeNodes: (nodeIds: string[]) => void | Promise<void>;
  onReassignImport?: (importId: string, scope: ProxyScope) => void | Promise<void>;
  onDeleteImport?: (importId: string) => void | Promise<void>;
  suggestedPort?: number | null;
  reallocatingImportId?: string | null;
  deletingImportId?: string | null;
  onOpenSessionByNode?: (payload: OpenSessionByNodeRequest) => void | Promise<void>;
  onOpenBatchByNode?: (payload: OpenBatchByNodeRequest) => void | Promise<void>;
  openingSessionNodeId?: string | null;
  openingBatch?: boolean;
}) {
  const { formatNumber, locale, t } = useI18n();
  const groups = proxyCatalog?.groups ?? [];
  const [expandedImportIds, setExpandedImportIds] = useState<string[]>([]);
  const [selectedNodeIds, setSelectedNodeIds] = useState<string[]>([]);
  const [singleDialogNodeId, setSingleDialogNodeId] = useState<string | null>(null);
  const [batchDialogOpen, setBatchDialogOpen] = useState(false);
  const [pendingDeleteImportId, setPendingDeleteImportId] = useState<string | null>(null);

  useEffect(() => {
    setExpandedImportIds(groups.map((group) => group.import.import_id));
    const validNodeIds = new Set(
      groups.flatMap((group) => group.nodes.map((node) => node.node_id)),
    );
    setSelectedNodeIds((current) => current.filter((nodeId) => validNodeIds.has(nodeId)));
  }, [groups]);

  const toggleGroup = (importId: string) => {
    setExpandedImportIds((current) =>
      current.includes(importId)
        ? current.filter((value) => value !== importId)
        : [...current, importId],
    );
  };

  const toggleNode = (nodeId: string, checked: boolean) => {
    setSelectedNodeIds((current) => {
      const next = new Set(current);
      if (checked) {
        next.add(nodeId);
      } else {
        next.delete(nodeId);
      }
      return [...next];
    });
  };

  const toggleGroupSelection = (group: ProxyCatalogGroupItem, checked: boolean) => {
    setSelectedNodeIds((current) => {
      const next = new Set(current);
      for (const node of group.nodes) {
        if (checked) {
          next.add(node.node_id);
        } else {
          next.delete(node.node_id);
        }
      }
      return [...next];
    });
  };

  const allNodeCount = groups.reduce((sum, group) => sum + group.nodes.length, 0);
  const nodeMap = new Map(
    groups.flatMap((group) => group.nodes.map((node) => [node.node_id, node] as const)),
  );
  const selectedNodes = selectedNodeIds
    .map((nodeId) => nodeMap.get(nodeId))
    .filter((node): node is ProxyCatalogNodeItem => Boolean(node));
  const singleDialogNode = singleDialogNodeId ? (nodeMap.get(singleDialogNodeId) ?? null) : null;
  const pendingDeleteImport = pendingDeleteImportId
    ? (groups.find((group) => group.import.import_id === pendingDeleteImportId)?.import ?? null)
    : null;

  return (
    <>
      {proxyCatalogError ? (
        <ActionResponsePanel
          title={t("Proxy catalog unavailable")}
          description={proxyCatalogError}
          tone="error"
        />
      ) : null}

      <DataTablePanel
        eyebrow={mode === "global" ? t("Subscription groups") : t("Available grouped nodes")}
        title={mode === "global" ? t("Grouped proxy catalog") : t("Current profile grouped nodes")}
        description={
          mode === "global"
            ? t(
                "Every import expands into its current child nodes here. Batch refresh and probe actions work on the selected nodes, while allocation and delete still operate at the import group level.",
              )
            : t(
                "The current profile shows its effective winner nodes grouped by import. Refresh, probe, and create-session actions all work directly from this list.",
              )
        }
        chips={[
          t(allNodeCount === 1 ? "{count} node" : "{count} nodes", {
            count: formatNumber(allNodeCount),
          }),
          t("Live stream: {state}", { state: liveConnectionState }),
        ]}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <Badge
              variant="outline"
              className="rounded-full px-2.5 py-0.5 font-mono text-[10px] uppercase tracking-[0.16em]"
            >
              <Layers3Icon className="mr-1 size-3.5" />
              {proxyCatalogLoading ? t("loading inventory") : t("imports live")}
            </Badge>
          </div>
        }
      >
        <div className="space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-[16px] border border-dashed border-border/70 bg-muted/10 px-3 py-2 text-xs text-muted-foreground">
            <span>
              {selectedNodeIds.length > 0
                ? t("Selected {count} nodes", { count: formatNumber(selectedNodeIds.length) })
                : t("Select one or more nodes to run batch operations.")}
            </span>
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                size="sm"
                disabled={selectedNodeIds.length === 0 || queueingOperation}
                onClick={() => void onRefreshNodes(selectedNodeIds)}
              >
                <RefreshCcwIcon className="size-3.5" />
                {t("Refresh selected")}
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={selectedNodeIds.length === 0 || queueingOperation}
                onClick={() => void onProbeNodes(selectedNodeIds)}
              >
                <ActivityIcon className="size-3.5" />
                {t("Probe selected")}
              </Button>
              {mode === "profile" ? (
                <Button
                  size="sm"
                  disabled={selectedNodeIds.length === 0 || !onOpenBatchByNode || openingBatch}
                  onClick={() => setBatchDialogOpen(true)}
                >
                  <PlusIcon className="size-3.5" />
                  {t("Create sessions")}
                </Button>
              ) : null}
            </div>
          </div>

          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-10 px-3" />
                <TableHead className="px-3 text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                  {t("Name")}
                </TableHead>
                <TableHead className="px-3 text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                  {t("Details")}
                </TableHead>
                <TableHead className="px-3 text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                  {t("Status")}
                </TableHead>
                <TableHead className="px-3 text-right text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                  {t("Actions")}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {groups.length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={5}
                    className="px-3 py-8 text-center text-sm text-muted-foreground"
                  >
                    {proxyCatalogLoading
                      ? t("Loading grouped proxy catalog...")
                      : t("No grouped nodes yet. Import a source first.")}
                  </TableCell>
                </TableRow>
              ) : (
                groups.map((group) => {
                  const expanded = expandedImportIds.includes(group.import.import_id);
                  const groupNodeIds = group.nodes.map((node) => node.node_id);
                  const selectedCount = groupNodeIds.filter((nodeId) =>
                    selectedNodeIds.includes(nodeId),
                  ).length;
                  const allSelected =
                    group.nodes.length > 0 && selectedCount === group.nodes.length;
                  const sourceTitle = formatImportSourceTitle(group.import);
                  const quotaSummary = formatSubscriptionQuota(
                    locale,
                    t,
                    group.import.subscription_metadata,
                  );
                  const expireSummary = formatSubscriptionExpire(
                    locale,
                    t,
                    group.import.subscription_metadata,
                  );
                  const canDeleteProfileImport =
                    mode === "profile" &&
                    group.import.source_scope.type === "profile" &&
                    group.import.source_scope.profile_id === currentProfileId;

                  return (
                    <Fragment key={group.import.import_id}>
                      <TableRow key={group.import.import_id} className="bg-muted/10">
                        <TableCell className="px-3 align-top">
                          <Checkbox
                            checked={allSelected}
                            onCheckedChange={(checked) =>
                              toggleGroupSelection(group, checked === true)
                            }
                            aria-label={t("Select import group {name}", {
                              name: formatImportLabel(group.import),
                            })}
                          />
                        </TableCell>
                        <TableCell className="px-3 py-3 align-top">
                          <div className="flex items-start gap-2">
                            <Button
                              variant="ghost"
                              size="icon-xs"
                              onClick={() => toggleGroup(group.import.import_id)}
                              aria-label={expanded ? t("Collapse group") : t("Expand group")}
                            >
                              {expanded ? (
                                <ChevronDownIcon className="size-3.5" />
                              ) : (
                                <ChevronRightIcon className="size-3.5" />
                              )}
                            </Button>
                            <div className="space-y-1">
                              <div className="font-medium text-foreground">
                                {formatImportLabel(group.import)}
                              </div>
                              {sourceTitle ? (
                                <div className="text-xs text-muted-foreground">
                                  {t("Source title: {title}", { title: sourceTitle })}
                                </div>
                              ) : null}
                              <div className="flex flex-wrap gap-1">
                                <Badge
                                  variant="secondary"
                                  className="rounded-full px-2 py-0.5 text-[10px]"
                                >
                                  {formatImportKind(group.import.import_kind, t)}
                                </Badge>
                                <Badge
                                  variant="outline"
                                  className="rounded-full px-2 py-0.5 text-[10px]"
                                >
                                  {formatScopeLabel(group.import.source_scope, t)}
                                </Badge>
                              </div>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell className="px-3 py-3 align-top">
                          <div className="space-y-1 text-xs leading-5 text-muted-foreground">
                            <div>
                              {t("{count} proxy", {
                                count: formatNumber(group.import.proxy_count),
                              })}{" "}
                              ·{" "}
                              {t("{count} IP", {
                                count: formatNumber(group.import.distinct_ip_count),
                              })}
                            </div>
                            {quotaSummary ? <div>{quotaSummary}</div> : null}
                            {expireSummary ? <div>{expireSummary}</div> : null}
                            <div>{formatTimestamp(locale, t, group.import.updated_at)}</div>
                            {mode === "profile" ? (
                              <InventoryProfiles
                                effectiveProfileIds={group.import.effective_profile_ids}
                              />
                            ) : null}
                          </div>
                        </TableCell>
                        <TableCell className="px-3 py-3 align-top">
                          <div className="space-y-1 text-xs leading-5 text-muted-foreground">
                            <div>
                              {t("Selected {count} nodes", { count: formatNumber(selectedCount) })}
                            </div>
                            <div>{t("Batch actions stay node-scoped.")}</div>
                          </div>
                        </TableCell>
                        <TableCell className="px-3 py-3 align-top text-right">
                          {mode === "global" ? (
                            <div className="flex flex-wrap justify-end gap-2">
                              <Select
                                disabled={
                                  !onReassignImport ||
                                  reallocatingImportId === group.import.import_id ||
                                  deletingImportId === group.import.import_id
                                }
                                value={encodeScope(group.import.allocation_scope)}
                                onValueChange={(value) => {
                                  void onReassignImport?.(
                                    group.import.import_id,
                                    decodeScope(value),
                                  );
                                }}
                              >
                                <SelectTrigger
                                  size="sm"
                                  className="h-8 w-[156px] bg-background text-xs"
                                >
                                  <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                  <SelectItem value="global">{t("Global pool")}</SelectItem>
                                  {profiles.map((candidateProfileId) => (
                                    <SelectItem
                                      key={candidateProfileId}
                                      value={`profile:${candidateProfileId}`}
                                    >
                                      {t("Profile {profileId}", { profileId: candidateProfileId })}
                                    </SelectItem>
                                  ))}
                                </SelectContent>
                              </Select>
                              <Button
                                variant="destructive"
                                size="sm"
                                disabled={
                                  !onDeleteImport || deletingImportId === group.import.import_id
                                }
                                onClick={() => setPendingDeleteImportId(group.import.import_id)}
                              >
                                <Trash2Icon className="size-3.5" />
                                {t("Delete")}
                              </Button>
                            </div>
                          ) : canDeleteProfileImport ? (
                            <div className="flex flex-wrap justify-end gap-2">
                              <Button
                                variant="destructive"
                                size="sm"
                                disabled={
                                  !onDeleteImport || deletingImportId === group.import.import_id
                                }
                                onClick={() => setPendingDeleteImportId(group.import.import_id)}
                              >
                                <Trash2Icon className="size-3.5" />
                                {t("Delete")}
                              </Button>
                            </div>
                          ) : (
                            <span className="text-xs text-muted-foreground">—</span>
                          )}
                        </TableCell>
                      </TableRow>

                      {expanded
                        ? group.nodes.map((node) => (
                            <TableRow key={node.node_id} className="bg-background/60">
                              <TableCell className="px-3 align-top">
                                <Checkbox
                                  checked={selectedNodeIds.includes(node.node_id)}
                                  onCheckedChange={(checked) =>
                                    toggleNode(node.node_id, checked === true)
                                  }
                                  aria-label={t("Select node {name}", { name: node.proxy_name })}
                                />
                              </TableCell>
                              <TableCell className="px-3 py-3 align-top">
                                <div className="space-y-1 pl-8">
                                  <div className="font-medium text-foreground">
                                    {node.proxy_name}
                                  </div>
                                  <div className="flex flex-wrap gap-1">
                                    <Badge
                                      variant="outline"
                                      className="rounded-full px-2 py-0.5 text-[10px]"
                                    >
                                      {node.proxy_type}
                                    </Badge>
                                    {mode === "global" ? (
                                      <Badge
                                        variant="outline"
                                        className="rounded-full px-2 py-0.5 text-[10px]"
                                      >
                                        {formatScopeLabel(node.allocation_scope, t)}
                                      </Badge>
                                    ) : null}
                                  </div>
                                </div>
                              </TableCell>
                              <TableCell className="px-3 py-3 align-top">
                                <div className="space-y-1 text-xs leading-5 text-muted-foreground">
                                  <div>{node.primary_ip ?? t("No resolved IPs")}</div>
                                  <div className="font-mono">{node.server}</div>
                                  {mode === "global" ? (
                                    <InventoryProfiles
                                      effectiveProfileIds={node.effective_profile_ids}
                                    />
                                  ) : null}
                                </div>
                              </TableCell>
                              <TableCell className="px-3 py-3 align-top">
                                <NodeStatusCell
                                  node={node}
                                  liveState={liveNodeStates[node.node_id]}
                                />
                              </TableCell>
                              <TableCell className="px-3 py-3 align-top text-right">
                                {mode === "profile" ? (
                                  <Button
                                    size="sm"
                                    disabled={
                                      !node.can_open_session ||
                                      !onOpenSessionByNode ||
                                      openingSessionNodeId === node.node_id ||
                                      openingBatch
                                    }
                                    onClick={() => setSingleDialogNodeId(node.node_id)}
                                  >
                                    <PlusIcon className="size-3.5" />
                                    {t("Create session")}
                                  </Button>
                                ) : (
                                  <span className="text-xs text-muted-foreground">—</span>
                                )}
                              </TableCell>
                            </TableRow>
                          ))
                        : null}
                    </Fragment>
                  );
                })
              )}
            </TableBody>
          </Table>
        </div>
      </DataTablePanel>

      {mode === "profile" ? (
        <>
          <NodePinnedSessionDialog
            open={Boolean(singleDialogNode)}
            node={singleDialogNode}
            suggestedPort={suggestedPort}
            isPending={Boolean(
              singleDialogNode && openingSessionNodeId === singleDialogNode.node_id,
            )}
            onOpenChange={(nextOpen) => {
              if (!nextOpen) {
                setSingleDialogNodeId(null);
              }
            }}
            onSubmit={async (payload) => {
              await onOpenSessionByNode?.(payload);
              setSingleDialogNodeId(null);
            }}
          />
          <NodePinnedBatchDialog
            open={batchDialogOpen}
            nodes={selectedNodes}
            suggestedPort={suggestedPort}
            isPending={openingBatch}
            onOpenChange={setBatchDialogOpen}
            onSubmit={async (payload) => {
              await onOpenBatchByNode?.(payload);
              setBatchDialogOpen(false);
            }}
          />
        </>
      ) : null}

      <DeleteImportConfirmDialog
        open={Boolean(pendingDeleteImport)}
        item={pendingDeleteImport}
        isPending={Boolean(
          pendingDeleteImport && deletingImportId === pendingDeleteImport.import_id,
        )}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) {
            setPendingDeleteImportId(null);
          }
        }}
        onConfirm={async (importId) => {
          await onDeleteImport?.(importId);
          setPendingDeleteImportId(null);
        }}
      />
    </>
  );
}
