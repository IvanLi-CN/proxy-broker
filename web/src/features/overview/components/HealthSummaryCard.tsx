import { AlertTriangleIcon, HeartPulseIcon, RefreshCwIcon, RouterIcon } from "lucide-react";

import { TopMetricCard } from "@/components/TopMetricCard";

interface HealthSummaryCardProps {
  status: string;
  activeSessions: number;
  hasWarnings: boolean;
  loadedProxies?: number | null;
  refreshedIps?: number | null;
}

export function HealthSummaryCard({
  status,
  activeSessions,
  hasWarnings,
  loadedProxies,
  refreshedIps,
}: HealthSummaryCardProps) {
  return (
    <section className="space-y-4">
      <div className="space-y-2">
        <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
          Command strip
        </div>
        <div className="max-w-3xl text-sm leading-6 text-muted-foreground md:text-[15px]">
          Read this row before you touch the pool. It surfaces service state, live listener load,
          the latest ingest pulse, and whether follow-up is waiting in the queue.
        </div>
      </div>
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <TopMetricCard
          title="Service pulse"
          value={status.toUpperCase()}
          description="Polled from /healthz every 10 seconds so the shell stays honest."
          icon={HeartPulseIcon}
          tone={status === "ok" ? "positive" : "warning"}
        />
        <TopMetricCard
          title="Live listeners"
          value={String(activeSessions)}
          description="Sessions currently consuming the active profile."
          icon={RouterIcon}
        />
        <TopMetricCard
          title="Pool inventory"
          value={loadedProxies == null ? "--" : String(loadedProxies)}
          description="Most recent successful subscription load reflected in the runway."
          icon={RefreshCwIcon}
        />
        <TopMetricCard
          title="Attention queue"
          value={hasWarnings ? "Review" : refreshedIps == null ? "Clear" : `${refreshedIps}`}
          description={
            hasWarnings
              ? "Warnings are waiting. Review them before opening long-lived listeners."
              : refreshedIps == null
                ? "No warnings are queued right now."
                : "Latest refresh completed cleanly and updated probe metadata."
          }
          icon={AlertTriangleIcon}
          tone={hasWarnings ? "warning" : "positive"}
        />
      </div>
    </section>
  );
}
