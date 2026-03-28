import { CircleAlertIcon, CircleCheckBigIcon, SparklesIcon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { AccessControlCard } from "@/features/overview/components/AccessControlCard";
import { HealthSummaryCard } from "@/features/overview/components/HealthSummaryCard";
import { RefreshCard } from "@/features/overview/components/RefreshCard";
import { SubscriptionFormCard } from "@/features/overview/components/SubscriptionFormCard";
import { useI18n } from "@/i18n";
import { formatOperatorWarning } from "@/lib/format";
import type {
  ApiKeySummary,
  CreateApiKeyResponse,
  CurrentUserState,
  HealthResponse,
  LoadSubscriptionRequest,
  LoadSubscriptionResponse,
  RefreshRequest,
  RefreshResponse,
} from "@/lib/types";

interface OverviewPageProps {
  health: HealthResponse;
  activeSessions: number;
  loadResponse?: LoadSubscriptionResponse | null;
  loadError?: string | null;
  refreshResponse?: RefreshResponse | null;
  refreshError?: string | null;
  loadingSubscription: boolean;
  refreshing: boolean;
  currentUser: CurrentUserState;
  apiKeys?: ApiKeySummary[];
  latestCreatedApiKey?: CreateApiKeyResponse | null;
  apiKeysLoading?: boolean;
  apiKeysError?: string | null;
  creatingApiKey?: boolean;
  revokingApiKeyId?: string | null;
  onLoadSubscription: (payload: LoadSubscriptionRequest) => void | Promise<void>;
  onRefresh: (payload: RefreshRequest) => void | Promise<void>;
  onCreateApiKey: (name: string) => void | Promise<void>;
  onRevokeApiKey: (keyId: string) => void | Promise<void>;
}

export function OverviewPage({
  health,
  activeSessions,
  loadResponse,
  loadError,
  refreshResponse,
  refreshError,
  loadingSubscription,
  refreshing,
  currentUser,
  apiKeys = [],
  latestCreatedApiKey = null,
  apiKeysLoading = false,
  apiKeysError = null,
  creatingApiKey = false,
  revokingApiKeyId = null,
  onLoadSubscription,
  onRefresh,
  onCreateApiKey,
  onRevokeApiKey,
}: OverviewPageProps) {
  const { t } = useI18n();
  const hasWarnings = Boolean(loadResponse?.warnings.length);
  const operatorChecklist = [
    t("Load a new feed whenever the upstream provider changes or rotates nodes."),
    t("Refresh probes before extracting IPs if geo labels or latency look stale."),
    t("Warnings are operator hints: review them before opening long-lived sessions."),
  ];

  return (
    <div className="space-y-8">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">{t("Overview")}</h1>
      </header>

      <HealthSummaryCard
        status={health.status}
        activeSessions={activeSessions}
        hasWarnings={hasWarnings}
        loadedProxies={loadResponse?.loaded_proxies ?? null}
        refreshedIps={refreshResponse?.probed_ips ?? null}
      />

      <section className="grid gap-6 xl:grid-cols-[minmax(0,1.2fr)_360px]">
        <div className="space-y-6">
          <SubscriptionFormCard
            error={loadError}
            isPending={loadingSubscription}
            onSubmit={onLoadSubscription}
            response={loadResponse}
          />
          <RefreshCard
            error={refreshError}
            isPending={refreshing}
            onSubmit={onRefresh}
            response={refreshResponse}
          />
        </div>

        <div className="space-y-6">
          <Card className="border-border/70 bg-card/96 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
            <CardHeader className="space-y-3 border-b border-border/70 pb-5">
              <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
                {t("Operator checklist")}
              </div>
              <CardTitle className="text-xl tracking-tight">
                {t("What to verify before touching the pool")}
              </CardTitle>
              <CardDescription className="text-sm leading-6 text-muted-foreground">
                {t(
                  "Keep this rail practical. It should help you decide what to do next without stealing focus from the two primary actions.",
                )}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4 pt-6">
              {operatorChecklist.map((item) => (
                <div
                  key={item}
                  className="flex items-start gap-3 rounded-2xl border border-border/70 bg-muted/20 p-4"
                >
                  <SparklesIcon className="mt-0.5 size-4 shrink-0 text-primary" />
                  <p className="text-sm leading-6 text-muted-foreground">{item}</p>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card className="border-border/70 bg-card/96 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
            <CardHeader className="space-y-3 border-b border-border/70 pb-5">
              <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
                {t("Latest state")}
              </div>
              <CardTitle className="text-xl tracking-tight">{t("Run summary")}</CardTitle>
              <CardDescription className="text-sm leading-6 text-muted-foreground">
                {t(
                  "These notes help you tell whether the page is ready for extraction or still needs an extra review pass.",
                )}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4 pt-6">
              <div className="grid gap-3">
                <div className="rounded-2xl border border-border/70 bg-background/80 p-4 shadow-sm">
                  <div className="flex items-center gap-2 text-sm font-medium text-foreground">
                    <CircleCheckBigIcon className="size-4 text-emerald-500" />
                    {t("Service health polling is live")}
                  </div>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    {t("/healthz updates every 10 seconds and feeds the command strip above.")}
                  </p>
                </div>
                <div className="rounded-2xl border border-border/70 bg-background/80 p-4 shadow-sm">
                  <div className="flex items-center gap-2 text-sm font-medium text-foreground">
                    <CircleAlertIcon className="size-4 text-amber-500" />
                    {t("File-mode sources resolve on the host")}
                  </div>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    {t(
                      "Browser uploads are not involved here. Use a path visible to the Rust service.",
                    )}
                  </p>
                </div>
              </div>
              {loadResponse?.warnings.length ? (
                <ActionResponsePanel
                  title={t("Subscription warnings")}
                  tone="warning"
                  description={t(
                    "The backend loaded the subscription, but some records still need operator attention before you keep drilling down.",
                  )}
                  bullets={loadResponse.warnings.map((warning) =>
                    formatOperatorWarning(t, warning),
                  )}
                />
              ) : null}
              {loadResponse && !loadResponse.warnings.length ? (
                <Badge
                  variant="outline"
                  className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
                >
                  {t("latest load completed without warnings")}
                </Badge>
              ) : null}
            </CardContent>
          </Card>

          <AccessControlCard
            currentUser={currentUser}
            apiKeys={apiKeys}
            latestCreatedKey={latestCreatedApiKey}
            apiKeysLoading={apiKeysLoading}
            apiKeysError={apiKeysError}
            creatingApiKey={creatingApiKey}
            revokingKeyId={revokingApiKeyId}
            onCreateApiKey={onCreateApiKey}
            onRevokeApiKey={onRevokeApiKey}
          />
        </div>
      </section>
    </div>
  );
}
