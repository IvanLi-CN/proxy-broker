import { HeartPulseIcon, RefreshCwIcon, RouterIcon } from "lucide-react";

import { TopMetricCard } from "@/components/TopMetricCard";

interface HealthSummaryCardProps {
  status: string;
  activeSessions: number;
  hasWarnings: boolean;
}

export function HealthSummaryCard({ status, activeSessions, hasWarnings }: HealthSummaryCardProps) {
  return (
    <section className="space-y-4">
      <div className="space-y-2">
        <div className="text-xs font-semibold uppercase tracking-[0.3em] text-primary/80">
          Flight deck
        </div>
        <div className="max-w-3xl text-sm leading-6 text-muted-foreground md:text-[15px]">
          Read this strip before touching the pool. It tells you whether the service is healthy, how
          many listeners are already consuming the profile, and whether operator follow-up is
          waiting in the queue.
        </div>
      </div>
      <div className="grid gap-4 md:grid-cols-3">
        <TopMetricCard
          title="Service pulse"
          value={status.toUpperCase()}
          description="Polled from /healthz every 10 seconds so you can sanity-check the operator plane at a glance."
          icon={HeartPulseIcon}
          tone={status === "ok" ? "positive" : "warning"}
        />
        <TopMetricCard
          title="Active listeners"
          value={String(activeSessions)}
          description="Sessions currently carved out from the selected profile. Use this to avoid colliding with live traffic."
          icon={RouterIcon}
        />
        <TopMetricCard
          title="Attention queue"
          value={hasWarnings ? "Review" : "Clear"}
          description="Subscription warnings and probe skips surface here first, before they turn into bad extract results."
          icon={RefreshCwIcon}
          tone={hasWarnings ? "warning" : "positive"}
        />
      </div>
    </section>
  );
}
