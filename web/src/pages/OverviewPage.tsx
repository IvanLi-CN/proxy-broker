import { CircleAlertIcon, CircleCheckBigIcon, SparklesIcon } from "lucide-react";
import { Link } from "react-router-dom";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { RouteHero } from "@/components/RouteHero";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { WorkflowRail } from "@/components/WorkflowRail";
import { HealthSummaryCard } from "@/features/overview/components/HealthSummaryCard";
import { RefreshCard, type RefreshFormValues } from "@/features/overview/components/RefreshCard";
import {
  SubscriptionFormCard,
  type SubscriptionFormValues,
} from "@/features/overview/components/SubscriptionFormCard";
import type {
  HealthResponse,
  LoadSubscriptionRequest,
  LoadSubscriptionResponse,
  RefreshRequest,
  RefreshResponse,
} from "@/lib/types";

interface OverviewPageProps {
  health: HealthResponse;
  activeSessions: number;
  initialized: boolean;
  initializationLoading: boolean;
  profileId: string;
  poolInventory?: number | null;
  subscriptionFormValues: SubscriptionFormValues;
  onSubscriptionFormValuesChange: (values: SubscriptionFormValues) => void;
  loadResponse?: LoadSubscriptionResponse | null;
  loadError?: string | null;
  refreshFormValues: RefreshFormValues;
  onRefreshFormValuesChange: (values: RefreshFormValues) => void;
  refreshResponse?: RefreshResponse | null;
  refreshError?: string | null;
  loadingSubscription: boolean;
  refreshing: boolean;
  onLoadSubscription: (payload: LoadSubscriptionRequest) => void | Promise<void>;
  onRefresh: (payload: RefreshRequest) => void | Promise<void>;
}

const operatorChecklist = [
  "Load a new feed whenever the upstream provider changes or rotates nodes.",
  "Refresh probes before extracting IPs if geo labels or latency look stale.",
  "Warnings are operator hints: review them before opening long-lived sessions.",
];

const recommendedFlow = [
  {
    title: "Refresh inventory",
    description: "Load the newest upstream feed into the active profile before anything else.",
  },
  {
    title: "Re-probe the edges",
    description: "Update geo and latency metadata so the next extract is based on current facts.",
  },
  {
    title: "Drill down with intent",
    description: "Extract candidates, then open only the listeners that still look worth holding.",
  },
];

export function OverviewPage({
  health,
  activeSessions,
  initialized,
  initializationLoading,
  profileId,
  poolInventory,
  subscriptionFormValues,
  onSubscriptionFormValuesChange,
  loadResponse,
  loadError,
  refreshFormValues,
  onRefreshFormValuesChange,
  refreshResponse,
  refreshError,
  loadingSubscription,
  refreshing,
  onLoadSubscription,
  onRefresh,
}: OverviewPageProps) {
  const hasWarnings = Boolean(loadResponse?.warnings.length);

  return (
    <div className="space-y-8">
      <RouteHero
        eyebrow="Overview"
        title="Run the operator plane like a control room, not a note pile."
        description="This surface keeps the pool ingest path, health state, and next-step guidance visible at the same time so you can move from feed refresh to listener orchestration without second-guessing the basics."
        badges={[
          {
            label: initializationLoading
              ? `loading ${profileId}`
              : initialized
                ? `${profileId} ready`
                : `${profileId} needs setup`,
            tone: initializationLoading ? "warning" : initialized ? "positive" : "warning",
          },
          { label: `${activeSessions} active sessions`, tone: "neutral" },
          {
            label: hasWarnings
              ? `${loadResponse?.warnings.length ?? 0} warnings queued`
              : "warnings clear",
            tone: hasWarnings ? "warning" : "positive",
          },
        ]}
        aside={
          <WorkflowRail eyebrow="Run order" title="Keep the runway clean" steps={recommendedFlow} />
        }
      />

      {!initializationLoading && !initialized ? (
        <ActionResponsePanel
          title="This project is not initialized yet"
          description="Load a subscription feed for this project first. Once the pool exists, refresh, extract, and session workflows will light up automatically."
          tone="warning"
          bullets={[
            "Use the subscription card below to create the project inventory.",
            "The project ID is already active, so the first successful load will persist it to the backend.",
          ]}
        />
      ) : null}

      <HealthSummaryCard
        status={health.status}
        activeSessions={activeSessions}
        hasWarnings={hasWarnings}
        initialized={initialized}
        loadedProxies={poolInventory ?? null}
        refreshedIps={refreshResponse?.probed_ips ?? null}
      />

      <section className="grid gap-6 xl:grid-cols-[minmax(0,1.2fr)_360px]">
        <div className="space-y-6">
          <SubscriptionFormCard
            error={loadError}
            initialValues={subscriptionFormValues}
            isPending={loadingSubscription}
            onSubmit={onLoadSubscription}
            onValuesChange={onSubscriptionFormValuesChange}
            response={loadResponse}
          />
          <RefreshCard
            error={refreshError}
            initialValues={refreshFormValues}
            isPending={refreshing}
            onSubmit={onRefresh}
            onValuesChange={onRefreshFormValuesChange}
            response={refreshResponse}
          />
        </div>

        <div className="space-y-6">
          <Card className="border-border/70 bg-card/96 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
            <CardHeader className="space-y-3 border-b border-border/70 pb-5">
              <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
                Operator checklist
              </div>
              <CardTitle className="text-xl tracking-tight">
                What to verify before touching the pool
              </CardTitle>
              <CardDescription className="text-sm leading-6 text-muted-foreground">
                Keep this rail practical. It should help you decide what to do next without stealing
                focus from the two primary actions.
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
                Latest state
              </div>
              <CardTitle className="text-xl tracking-tight">Run summary</CardTitle>
              <CardDescription className="text-sm leading-6 text-muted-foreground">
                These notes help you tell whether the page is ready for extraction or still needs an
                extra review pass.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4 pt-6">
              <div className="grid gap-3">
                <div className="rounded-2xl border border-border/70 bg-background/80 p-4 shadow-sm">
                  <div className="flex items-center gap-2 text-sm font-medium text-foreground">
                    <CircleCheckBigIcon className="size-4 text-emerald-500" />
                    Service health polling is live
                  </div>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    /healthz updates every 10 seconds and feeds the command strip above.
                  </p>
                </div>
                <div className="rounded-2xl border border-border/70 bg-background/80 p-4 shadow-sm">
                  <div className="flex items-center gap-2 text-sm font-medium text-foreground">
                    <CircleAlertIcon className="size-4 text-amber-500" />
                    File-mode sources resolve on the host
                  </div>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    Browser uploads are not involved here. Use a path visible to the Rust service.
                  </p>
                </div>
              </div>
              {!initialized && !initializationLoading ? (
                <div className="rounded-2xl border border-amber-500/30 bg-amber-500/8 p-4 text-sm leading-6 text-muted-foreground">
                  This project does not have a pool yet. Load a subscription below, then move on to
                  <Button asChild variant="link" className="h-auto px-1 align-baseline">
                    <Link to="/ips">IP Extract</Link>
                  </Button>
                  and
                  <Button asChild variant="link" className="h-auto px-1 align-baseline">
                    <Link to="/sessions">Sessions</Link>
                  </Button>
                  once the inventory exists.
                </div>
              ) : null}
              {loadResponse?.warnings.length ? (
                <ActionResponsePanel
                  title="Subscription warnings"
                  tone="warning"
                  description="The backend loaded the subscription, but some records still need operator attention before you keep drilling down."
                  bullets={loadResponse.warnings}
                />
              ) : null}
              {loadResponse && !loadResponse.warnings.length ? (
                <Badge
                  variant="outline"
                  className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
                >
                  latest load completed without warnings
                </Badge>
              ) : null}
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
