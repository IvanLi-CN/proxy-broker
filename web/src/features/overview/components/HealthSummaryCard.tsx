import { HeartPulseIcon, RefreshCwIcon, RouterIcon } from "lucide-react";

import { TopMetricCard } from "@/components/TopMetricCard";

interface HealthSummaryCardProps {
  status: string;
  activeSessions: number;
  hasWarnings: boolean;
}

export function HealthSummaryCard({ status, activeSessions, hasWarnings }: HealthSummaryCardProps) {
  return (
    <div className="grid gap-4 md:grid-cols-3">
      <TopMetricCard
        title="Service"
        value={status.toUpperCase()}
        description="Derived from /healthz and refreshed every 10s."
        icon={HeartPulseIcon}
        tone={status === "ok" ? "positive" : "warning"}
      />
      <TopMetricCard
        title="Open sessions"
        value={String(activeSessions)}
        description="Active listeners stored for the selected profile."
        icon={RouterIcon}
      />
      <TopMetricCard
        title="Operator notes"
        value={hasWarnings ? "Review" : "Clear"}
        description="Subscription warnings and probe skips bubble up here first."
        icon={RefreshCwIcon}
        tone={hasWarnings ? "warning" : "positive"}
      />
    </div>
  );
}
