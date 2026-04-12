import { zodResolver } from "@hookform/resolvers/zod";
import { FolderSyncIcon, Layers3Icon, Settings2Icon, Trash2Icon } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { DataTablePanel } from "@/components/DataTablePanel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
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
import { useI18n } from "@/i18n";
import { formatOperatorWarning } from "@/lib/format";
import type {
  CurrentUserState,
  ListProxyInventoryResponse,
  LoadSubscriptionRequest,
  LoadSubscriptionResponse,
  ProfileProxySettings,
  ProxyScope,
} from "@/lib/types";
import { cn } from "@/lib/utils";

const loadCardSchema = z.object({
  sourceType: z.enum(["url", "file"]),
  sourceValue: z.string().trim().min(1, "validation.source_value_required"),
});

type LoadCardFormValues = z.infer<typeof loadCardSchema>;

interface ProxyLoadCardProps {
  eyebrow: string;
  title: string;
  description: string;
  scopeChip: string;
  pending: boolean;
  response?: LoadSubscriptionResponse | null;
  error?: string | null;
  defaultValue: string;
  submitLabel: string;
  successTitle: string;
  successDescription: string;
  onSubmit: (payload: LoadSubscriptionRequest) => void | Promise<void>;
}

function ProxyLoadCard({
  eyebrow,
  title,
  description,
  scopeChip,
  pending,
  response,
  error,
  defaultValue,
  submitLabel,
  successTitle,
  successDescription,
  onSubmit,
}: ProxyLoadCardProps) {
  const { t } = useI18n();
  const form = useForm<LoadCardFormValues>({
    resolver: zodResolver(loadCardSchema),
    defaultValues: {
      sourceType: "url",
      sourceValue: defaultValue,
    },
  });
  const sourceType = form.watch("sourceType");

  return (
    <Card className="overflow-hidden border-border/70 bg-card/96 shadow-[0_20px_60px_-40px_rgba(15,23,42,0.5)]">
      <CardHeader className="gap-3 border-b border-border/70 bg-muted/15 pb-4">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div className="space-y-1.5">
            <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
              {eyebrow}
            </div>
            <CardTitle className="flex items-center gap-2 text-lg tracking-tight md:text-xl">
              <FolderSyncIcon className="size-4.5 text-primary" />
              {title}
            </CardTitle>
            <CardDescription className="max-w-xl text-sm leading-5 text-muted-foreground">
              {description}
            </CardDescription>
          </div>
          <div className="flex flex-wrap gap-1.5">
            <Badge
              variant="outline"
              className="rounded-full px-2.5 py-0.5 font-mono text-[10px] uppercase tracking-[0.16em]"
            >
              {scopeChip}
            </Badge>
            <Badge
              variant="outline"
              className="rounded-full px-2.5 py-0.5 font-mono text-[10px] uppercase tracking-[0.16em]"
            >
              {sourceType === "url" ? t("remote fetch") : t("host file")}
            </Badge>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4 pt-4">
        <form
          className="space-y-3"
          onSubmit={form.handleSubmit((values) =>
            onSubmit({
              source: {
                type: values.sourceType,
                value: values.sourceValue.trim(),
              },
            }),
          )}
        >
          <div className="grid gap-3 rounded-[20px] border border-border/70 bg-background/80 p-3 md:grid-cols-[168px_minmax(0,1fr)] xl:grid-cols-[168px_minmax(0,1fr)_auto] xl:items-start">
            <div className="space-y-2">
              <Label htmlFor={`${eyebrow}-source-type`}>{t("Source type")}</Label>
              <Controller
                control={form.control}
                name="sourceType"
                render={({ field }) => (
                  <Select onValueChange={field.onChange} value={field.value}>
                    <SelectTrigger
                      id={`${eyebrow}-source-type`}
                      size="lg"
                      className="w-full bg-card"
                    >
                      <SelectValue placeholder={t("Choose source type")} />
                    </SelectTrigger>
                    <SelectContent size="lg">
                      <SelectItem size="lg" value="url">
                        {t("URL")}
                      </SelectItem>
                      <SelectItem size="lg" value="file">
                        {t("File path")}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor={`${eyebrow}-source-value`}>{t("Value")}</Label>
              <Input
                id={`${eyebrow}-source-value`}
                size="lg"
                {...form.register("sourceValue")}
                placeholder="https://example.com/subscription.yaml"
                className="bg-card font-mono text-xs md:text-sm"
              />
            </div>
            <div className="flex xl:justify-end">
              <Button disabled={pending} size="lg" type="submit" className="w-full xl:min-w-40">
                {pending ? t("Loading subscription...") : submitLabel}
              </Button>
            </div>
          </div>

          {form.formState.errors.sourceValue ? (
            <p className="text-xs text-destructive" role="alert">
              {t(form.formState.errors.sourceValue.message ?? "validation.source_value_required")}
            </p>
          ) : (
            <div className="flex flex-wrap gap-x-3 gap-y-1 text-xs leading-5 text-muted-foreground">
              <span>
                {sourceType === "url"
                  ? t("Use the upstream subscription URL that the backend can fetch directly.")
                  : t("Provide a server-local path that the Rust process can read on disk.")}
              </span>
              <span className="hidden text-border md:inline">•</span>
              <span>{t("Re-import restores nodes that still exist upstream.")}</span>
            </div>
          )}
        </form>

        {response ? (
          <ActionResponsePanel
            title={successTitle}
            description={successDescription}
            tone={response.warnings.length > 0 ? "warning" : "success"}
            bullets={response.warnings.map((warning) => formatOperatorWarning(t, warning))}
          />
        ) : null}
        {error ? (
          <ActionResponsePanel title={t("Load failed")} description={error} tone="error" />
        ) : null}
      </CardContent>
    </Card>
  );
}

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

export interface ProxiesPageProps {
  profileId: string;
  profiles: string[];
  currentUser: CurrentUserState;
  accessDenied?: boolean;
  authError?: string | null;
  globalLoadResponse?: LoadSubscriptionResponse | null;
  globalLoadError?: string | null;
  profileLoadResponse?: LoadSubscriptionResponse | null;
  profileLoadError?: string | null;
  loadingGlobal: boolean;
  loadingProfile: boolean;
  inventory?: ListProxyInventoryResponse | null;
  inventoryLoading: boolean;
  inventoryError?: string | null;
  proxySettings?: ProfileProxySettings | null;
  proxySettingsLoading: boolean;
  proxySettingsError?: string | null;
  updatingSettings: boolean;
  reallocatingNodeId?: string | null;
  deletingNodeId?: string | null;
  onLoadGlobal: (payload: LoadSubscriptionRequest) => void | Promise<void>;
  onLoadProfile: (payload: LoadSubscriptionRequest) => void | Promise<void>;
  onToggleUseGlobalProxies: (nextValue: boolean) => void | Promise<void>;
  onReassignNode: (nodeId: string, scope: ProxyScope) => void | Promise<void>;
  onDeleteNode: (nodeId: string) => void | Promise<void>;
}

export function ProxiesPage({
  profileId,
  profiles,
  currentUser: _currentUser,
  accessDenied = false,
  authError = null,
  globalLoadResponse,
  globalLoadError,
  profileLoadResponse,
  profileLoadError,
  loadingGlobal,
  loadingProfile,
  inventory,
  inventoryLoading,
  inventoryError,
  proxySettings,
  proxySettingsLoading,
  proxySettingsError,
  updatingSettings,
  reallocatingNodeId = null,
  deletingNodeId = null,
  onLoadGlobal,
  onLoadProfile,
  onToggleUseGlobalProxies,
  onReassignNode,
  onDeleteNode,
}: ProxiesPageProps) {
  const { formatNumber, t } = useI18n();
  const items = inventory?.items ?? [];
  const useGlobalProxies = proxySettings?.use_global_proxies ?? true;

  if (authError) {
    return (
      <div className="space-y-5">
        <header>
          <h1 className="text-2xl font-semibold tracking-tight text-foreground">{t("Proxies")}</h1>
        </header>
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
        <header>
          <h1 className="text-2xl font-semibold tracking-tight text-foreground">{t("Proxies")}</h1>
        </header>
        <ActionResponsePanel
          title={t("Admin access required")}
          description={t(
            "The proxies workspace is restricted to the admin operator plane because it can change global pool allocation.",
          )}
          tone="error"
        />
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <header className="space-y-1.5">
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">{t("Proxies")}</h1>
        <p className="max-w-3xl text-sm leading-5 text-muted-foreground">
          {t(
            "Manage the global pool, the current profile's local imports, and where each imported node is allocated.",
          )}
        </p>
      </header>

      <section className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <ProxyLoadCard
          defaultValue="https://example.com/global-subscription.yaml"
          description={t(
            "Import one source into the shared global pool. Profiles that keep global usage enabled will inherit these nodes immediately.",
          )}
          error={globalLoadError}
          eyebrow={t("Global scope")}
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

        <ProxyLoadCard
          defaultValue="https://example.com/profile-subscription.yaml"
          description={t(
            "Import nodes for the current profile only. These nodes stay local unless you later reassign them from the inventory table.",
          )}
          error={profileLoadError}
          eyebrow={t("Current profile")}
          onSubmit={onLoadProfile}
          pending={loadingProfile}
          response={profileLoadResponse}
          scopeChip={t("allocation defaults to {profileId}", { profileId })}
          submitLabel={t("Import profile pool")}
          successDescription={t(
            "Imported {proxyCount} proxies across {ipCount} distinct IPs into profile {profileId}.",
            {
              proxyCount: profileLoadResponse?.loaded_proxies ?? 0,
              ipCount: profileLoadResponse?.distinct_ips ?? 0,
              profileId,
            },
          )}
          successTitle={t("Profile pool updated")}
          title={t("Import local pool for {profileId}", { profileId })}
        />
      </section>

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
        description={t("Track source scope, current allocation, and where each node is effective.")}
        chips={[
          t(items.length === 1 ? "{count} node" : "{count} nodes", {
            count: formatNumber(items.length),
          }),
          t("current profile {profileId}", { profileId }),
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
          <div className="grid gap-3 rounded-[20px] border border-border/70 bg-muted/10 p-3 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-center">
            <div className="space-y-1">
              <div className="flex flex-wrap items-center gap-2">
                <div className="flex items-center gap-2 text-sm font-medium text-foreground">
                  <Settings2Icon className="size-4 text-primary" />
                  {t("Use global pool for {profileId}", { profileId })}
                </div>
                <Badge
                  variant={useGlobalProxies ? "default" : "secondary"}
                  className={cn(
                    "rounded-full px-2 py-0.5 font-mono text-[10px] uppercase tracking-[0.16em]",
                    !useGlobalProxies && "bg-muted text-foreground",
                  )}
                >
                  {useGlobalProxies ? t("global enabled") : t("local-only")}
                </Badge>
              </div>
              <p className="text-xs leading-5 text-muted-foreground">
                {t(
                  "Turning this off immediately rebuilds the profile from local nodes only and removes sessions that depended on global-only nodes.",
                )}
              </p>
            </div>
            <div className="flex items-center gap-3 rounded-full border border-border/70 bg-background/80 px-3 py-2">
              <Checkbox
                id="use-global-proxies"
                checked={useGlobalProxies}
                disabled={proxySettingsLoading || updatingSettings}
                onCheckedChange={(checked) => {
                  void onToggleUseGlobalProxies(checked === true);
                }}
                aria-label={t("Use global pool for {profileId}", { profileId })}
              />
              <Label
                htmlFor="use-global-proxies"
                className="cursor-pointer text-sm font-medium text-foreground"
              >
                {t("Compose {profileId} from the global pool as well", { profileId })}
              </Label>
            </div>
          </div>

          {proxySettingsError ? (
            <ActionResponsePanel
              title={t("Profile proxy settings unavailable")}
              description={proxySettingsError}
              tone="error"
            />
          ) : null}

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
                      : t(
                          "No imported nodes yet. Use the cards above to seed the global or local pool.",
                        )}
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
                        <div className="flex justify-end gap-2">
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
                        </div>
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
