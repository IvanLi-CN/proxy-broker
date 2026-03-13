import { CircleAlertIcon, CircleCheckBigIcon, SparklesIcon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { HealthSummaryCard } from "@/features/overview/components/HealthSummaryCard";
import { RefreshCard } from "@/features/overview/components/RefreshCard";
import { SubscriptionFormCard } from "@/features/overview/components/SubscriptionFormCard";
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
  loadResponse?: LoadSubscriptionResponse | null;
  loadError?: string | null;
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
  "Load the latest subscription feed into the active profile.",
  "Refresh probes + geo hints so extract results use fresh metadata.",
  "Inspect IP extract results, then open the sessions you actually need.",
];

export function OverviewPage({
  health,
  activeSessions,
  loadResponse,
  loadError,
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
      <section className="grid gap-5 xl:grid-cols-[1.3fr_0.7fr]">
        <Card className="overflow-hidden border-border/70 bg-card shadow-sm">
          <CardHeader className="gap-6 border-b border-border/70 bg-[linear-gradient(135deg,rgba(15,23,42,0.04),rgba(14,165,233,0.08))] pb-6">
            <div className="flex flex-wrap items-center gap-2">
              <Badge
                variant="outline"
                className="rounded-full px-3 py-1 uppercase tracking-[0.24em]"
              >
                Subscription control
              </Badge>
              <Badge variant="secondary" className="rounded-full px-3 py-1">
                {health.status === "ok" ? "Service healthy" : "Needs attention"}
              </Badge>
              <Badge variant="secondary" className="rounded-full px-3 py-1">
                {activeSessions} active sessions
              </Badge>
            </div>
            <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_280px] lg:items-end">
              <div className="space-y-3">
                <CardTitle className="max-w-3xl text-3xl font-semibold tracking-tight text-foreground md:text-5xl">
                  Keep the pool fresh, then move through extraction with intent.
                </CardTitle>
                <CardDescription className="max-w-3xl text-sm leading-7 text-muted-foreground md:text-base">
                  This is the operator runway for the selected profile. Start by loading a clean
                  subscription feed, refresh probe metadata when upstream behavior shifts, and only
                  then carve out listeners from the pool.
                </CardDescription>
              </div>
              <div className="grid gap-3">
                <div className="rounded-2xl border border-border/70 bg-background/90 p-4 shadow-sm">
                  <div className="flex items-center gap-2 text-sm font-medium text-foreground">
                    <CircleCheckBigIcon className="size-4 text-emerald-500" />
                    Service health polling is live
                  </div>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    /healthz updates every 10 seconds and feeds the flight deck below.
                  </p>
                </div>
                <div className="rounded-2xl border border-border/70 bg-background/90 p-4 shadow-sm">
                  <div className="flex items-center gap-2 text-sm font-medium text-foreground">
                    <CircleAlertIcon className="size-4 text-amber-500" />
                    File-mode sources resolve on the host
                  </div>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    Browser uploads are not involved here. Use the path that the Rust service can
                    see locally.
                  </p>
                </div>
              </div>
            </div>
          </CardHeader>
        </Card>

        <Card className="border-border/70 bg-card/95 shadow-sm">
          <CardHeader className="space-y-3 border-b border-border/70 pb-5">
            <div className="text-xs font-semibold uppercase tracking-[0.3em] text-primary/80">
              Operator checklist
            </div>
            <CardTitle className="text-xl tracking-tight">
              What to verify before touching the pool
            </CardTitle>
            <CardDescription className="text-sm leading-6 text-muted-foreground">
              Keep this rail compact and practical. It should help you decide what to do next, not
              compete with the primary action.
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
      </section>

      <HealthSummaryCard
        status={health.status}
        activeSessions={activeSessions}
        hasWarnings={hasWarnings}
      />

      <section className="grid gap-6 xl:grid-cols-[minmax(0,1.2fr)_360px]">
        <div className="space-y-6">
          <SubscriptionFormCard
            error={loadError}
            isPending={loadingSubscription}
            onSubmit={onLoadSubscription}
            response={loadResponse}
          />
          {loadResponse?.warnings.length ? (
            <ActionResponsePanel
              title="Subscription warnings"
              tone="warning"
              description="The backend loaded the subscription, but some records still need operator attention before you keep drilling down."
              bullets={loadResponse.warnings}
            />
          ) : null}
        </div>

        <div className="space-y-6">
          <RefreshCard
            error={refreshError}
            isPending={refreshing}
            onSubmit={onRefresh}
            response={refreshResponse}
          />
          <Card className="border-border/70 bg-card/95 shadow-sm">
            <CardHeader className="space-y-3 border-b border-border/70 pb-5">
              <div className="text-xs font-semibold uppercase tracking-[0.3em] text-primary/80">
                Recommended flow
              </div>
              <CardTitle className="text-xl tracking-tight">
                Run the console in a clean order
              </CardTitle>
              <CardDescription className="text-sm leading-6 text-muted-foreground">
                This page should read like a workflow, not a document wall. Follow this sequence and
                you will avoid most empty extracts and stale-session surprises.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4 pt-6">
              {recommendedFlow.map((item, index) => (
                <div
                  key={item}
                  className="flex items-start gap-4 rounded-2xl border border-border/70 bg-muted/20 p-4"
                >
                  <div className="flex size-8 shrink-0 items-center justify-center rounded-full border border-border/70 bg-background font-mono text-sm font-semibold text-foreground">
                    {index + 1}
                  </div>
                  <p className="text-sm leading-6 text-muted-foreground">{item}</p>
                </div>
              ))}
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
