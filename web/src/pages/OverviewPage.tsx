import { ActionResponsePanel } from "@/components/ActionResponsePanel";
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
    <div className="space-y-6">
      <section className="grid gap-4 lg:grid-cols-[1.1fr_0.9fr]">
        <div className="space-y-3">
          <div className="text-xs font-semibold uppercase tracking-[0.28em] text-primary">
            Subscription control
          </div>
          <h1 className="max-w-2xl text-3xl font-semibold tracking-tight text-foreground md:text-4xl">
            Keep the proxy pool fresh before you carve out sessions.
          </h1>
          <p className="max-w-2xl text-sm leading-7 text-muted-foreground md:text-base">
            This console speaks directly to the Rust API. Start here whenever you need to load a new
            subscription feed, refresh probe metadata, or sanity-check the operator state before
            extracting IPs.
          </p>
        </div>
        <ActionResponsePanel
          title="Operator note"
          tone="warning"
          description="File-mode sources resolve on the service host, not in the browser sandbox. Double-check server paths before submitting."
          bullets={[
            "Health polling is read-only and updates every 10 seconds.",
            "Session count mirrors the currently selected profile only.",
          ]}
        />
      </section>

      <HealthSummaryCard
        status={health.status}
        activeSessions={activeSessions}
        hasWarnings={hasWarnings}
      />

      {loadResponse?.warnings.length ? (
        <ActionResponsePanel
          title="Subscription warnings"
          tone="warning"
          description="The backend loaded the subscription, but some records need operator attention."
          bullets={loadResponse.warnings}
        />
      ) : null}

      <section className="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
        <SubscriptionFormCard
          error={loadError}
          isPending={loadingSubscription}
          onSubmit={onLoadSubscription}
          response={loadResponse}
        />
        <div className="space-y-6">
          <RefreshCard
            error={refreshError}
            isPending={refreshing}
            onSubmit={onRefresh}
            response={refreshResponse}
          />
          <ActionResponsePanel
            title="Recommended loop"
            description="Load subscription → refresh probes → review IP extract → open sessions."
            bullets={[
              "Use refresh with force when upstream geo changed sharply.",
              "Treat warnings as hints, not hard failures, unless extraction starts returning empty sets.",
            ]}
          />
        </div>
      </section>
    </div>
  );
}
