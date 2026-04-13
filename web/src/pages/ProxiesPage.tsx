import { Layers3Icon, Trash2Icon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { DataTablePanel } from "@/components/DataTablePanel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
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
import { useI18n } from "@/i18n";
import type {
  CurrentUserState,
  ListProxyInventoryResponse,
  LoadSubscriptionRequest,
  LoadSubscriptionResponse,
  ProfileProxySettings,
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

interface GlobalProxiesPageProps {
  mode: "global";
  profiles: string[];
  currentUser: CurrentUserState;
  accessDenied?: boolean;
  authError?: string | null;
  globalLoadResponse?: LoadSubscriptionResponse | null;
  globalLoadError?: string | null;
  loadingGlobal: boolean;
  inventory?: ListProxyInventoryResponse | null;
  inventoryLoading: boolean;
  inventoryError?: string | null;
  reallocatingNodeId?: string | null;
  deletingNodeId?: string | null;
  onLoadGlobal: (payload: LoadSubscriptionRequest) => void | Promise<void>;
  onReassignNode: (nodeId: string, scope: ProxyScope) => void | Promise<void>;
  onDeleteNode: (nodeId: string) => void | Promise<void>;
}

interface ProfileProxiesPageProps {
  mode: "profile";
  profileId: string;
  currentUser: CurrentUserState;
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
  currentUser: _currentUser,
  accessDenied = false,
  authError = null,
  globalLoadResponse,
  globalLoadError,
  loadingGlobal,
  inventory,
  inventoryLoading,
  inventoryError,
  reallocatingNodeId = null,
  deletingNodeId = null,
  onLoadGlobal,
  onReassignNode,
  onDeleteNode,
}: GlobalProxiesPageProps) {
  const { formatNumber, t } = useI18n();
  const items = inventory?.items ?? [];

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
          "Import one upstream into the shared pool. Profiles that keep global usage enabled inherit these nodes immediately.",
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

      {inventoryError ? (
        <ActionResponsePanel
          title={t("Proxy inventory unavailable")}
          description={inventoryError}
          tone="error"
        />
      ) : null}

      <DataTablePanel
        eyebrow={t("Unified inventory")}
        title={t("Global pool and profile allocations")}
        description={t(
          "See where each imported node came from, where it is allocated now, and which profiles currently inherit it.",
        )}
        chips={[
          t(items.length === 1 ? "{count} node" : "{count} nodes", {
            count: formatNumber(items.length),
          }),
        ]}
        actions={
          <Badge
            variant="outline"
            className="rounded-full px-2.5 py-0.5 font-mono text-[10px] uppercase tracking-[0.16em]"
          >
            <Layers3Icon className="mr-1 size-3.5" />
            {inventoryLoading ? t("loading inventory") : t("inventory live")}
          </Badge>
        }
      >
        <div className="space-y-3">
          <div className="rounded-[16px] border border-dashed border-border/70 bg-muted/10 px-3 py-2 text-xs leading-5 text-muted-foreground">
            {t(
              "Deleting or reallocating an imported node only affects the current inventory snapshot. The next source reload restores anything the upstream still contains.",
            )}
          </div>

          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="h-10 px-3 text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                  {t("Proxy")}
                </TableHead>
                <TableHead className="h-10 px-3 text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                  {t("Source scope")}
                </TableHead>
                <TableHead className="h-10 px-3 text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                  {t("Allocation scope")}
                </TableHead>
                <TableHead className="h-10 px-3 text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                  {t("Effective profiles")}
                </TableHead>
                <TableHead className="h-10 px-3 text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                  {t("Resolved IPs")}
                </TableHead>
                <TableHead className="h-10 px-3 text-right text-[11px] uppercase tracking-[0.14em] text-muted-foreground">
                  {t("Actions")}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {items.length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={6}
                    className="px-3 py-8 text-center text-sm text-muted-foreground"
                  >
                    {inventoryLoading
                      ? t("Loading proxy inventory...")
                      : t("No imported nodes yet. Load the global pool first.")}
                  </TableCell>
                </TableRow>
              ) : (
                items.map((item) => {
                  const pending =
                    reallocatingNodeId === item.node_id || deletingNodeId === item.node_id;

                  return (
                    <TableRow key={item.node_id}>
                      <TableCell className="px-3 py-3 align-top">
                        <div className="space-y-0.5">
                          <div className="font-medium text-foreground">{item.proxy_name}</div>
                          <div className="font-mono text-xs text-muted-foreground">
                            {item.proxy_type} · {item.server}
                          </div>
                        </div>
                      </TableCell>
                      <TableCell className="px-3 py-3 align-top">
                        <Badge variant="outline" className="rounded-full px-2 py-0.5 text-[10px]">
                          {formatScopeLabel(item.source_scope, t)}
                        </Badge>
                      </TableCell>
                      <TableCell className="px-3 py-3 align-top">
                        <Select
                          disabled={pending}
                          value={encodeScope(item.allocation_scope)}
                          onValueChange={(value) => {
                            void onReassignNode(item.node_id, decodeScope(value));
                          }}
                        >
                          <SelectTrigger size="sm" className="h-8 w-[156px] bg-background text-xs">
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
                      </TableCell>
                      <TableCell className="px-3 py-3 align-top">
                        <InventoryProfiles effectiveProfileIds={item.effective_profile_ids} />
                      </TableCell>
                      <TableCell className="px-3 py-3 align-top">
                        <div className="max-w-[240px] whitespace-normal text-[11px] leading-5 text-muted-foreground">
                          {item.resolved_ips.length > 0
                            ? item.resolved_ips.join(", ")
                            : t("No resolved IPs")}
                        </div>
                      </TableCell>
                      <TableCell className="px-3 py-3 align-top text-right">
                        <Button
                          variant="destructive"
                          size="sm"
                          className="h-8 px-2.5 text-xs"
                          disabled={pending}
                          onClick={() => {
                            void onDeleteNode(item.node_id);
                          }}
                        >
                          <Trash2Icon className="size-4" />
                          {deletingNodeId === item.node_id ? t("Deleting...") : t("Delete")}
                        </Button>
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </div>
      </DataTablePanel>
    </div>
  );
}

function ProfileProxiesView({
  profileId,
  currentUser: _currentUser,
  profileLoadResponse,
  profileLoadError,
  loadingProfile,
  proxySettings,
  proxySettingsLoading = false,
  proxySettingsError = null,
  updatingSettings = false,
  showProxyPolicy = true,
  onLoadProfile,
  onToggleUseGlobalProxies,
}: ProfileProxiesPageProps) {
  const { t } = useI18n();

  return (
    <div className="space-y-5">
      <PageHeader
        scopeBadge={profileId}
        title={t("Proxy")}
        description={t(
          "Manage local imports and whether {profileId} also composes the global pool.",
          {
            profileId,
          },
        )}
      />
      <section className="grid gap-5 xl:grid-cols-[minmax(0,1.55fr)_minmax(320px,0.9fr)]">
        <ProxyLoadCard
          defaultValue="https://example.com/profile-subscription.yaml"
          description={t(
            "Import nodes for this profile only. They stay local unless you later reassign them from the global config.",
          )}
          error={profileLoadError}
          eyebrow={t("Current profile")}
          onSubmit={onLoadProfile}
          pending={loadingProfile}
          response={profileLoadResponse}
          scopeChip={t("allocation defaults to {profileId}", { profileId })}
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

        <div className="space-y-4">
          {showProxyPolicy ? (
            <ProfileProxyPolicyCard
              profileId={profileId}
              proxySettingsError={proxySettingsError}
              proxySettingsLoading={proxySettingsLoading}
              updatingSettings={updatingSettings}
              useGlobalProxies={proxySettings?.use_global_proxies ?? true}
              onToggleUseGlobalProxies={onToggleUseGlobalProxies}
            />
          ) : null}

          <div className="rounded-[18px] border border-dashed border-border/70 bg-muted/10 px-4 py-3 text-xs leading-5 text-muted-foreground">
            {t(
              "Cross-profile allocation and node deletion are only available after switching the current config to Global.",
            )}
          </div>
        </div>
      </section>
    </div>
  );
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
  const { t } = useI18n();

  return (
    <header className="space-y-1.5">
      <div className="flex flex-wrap items-center gap-2">
        <div className="text-[11px] font-semibold uppercase tracking-[0.28em] text-primary/80">
          {t("Proxy")}
        </div>
        <Badge
          variant="outline"
          className="rounded-full px-2 py-0.5 font-mono text-[10px] uppercase tracking-[0.16em]"
        >
          {scopeBadge}
        </Badge>
      </div>
      <h1 className="text-2xl font-semibold tracking-tight text-foreground">{title}</h1>
      <p className="max-w-3xl text-sm leading-5 text-muted-foreground">{description}</p>
    </header>
  );
}
