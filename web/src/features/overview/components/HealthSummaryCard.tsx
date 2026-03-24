import { AlertTriangleIcon, HeartPulseIcon, RefreshCwIcon, RouterIcon } from "lucide-react";

import { TopMetricCard } from "@/components/TopMetricCard";
import { useI18n } from "@/i18n";
import { formatHealthStatus } from "@/lib/format";

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
  const { formatNumber, t } = useI18n();

  return (
    <section className="space-y-4">
      <div className="space-y-2">
        <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
          {t("Command strip")}
        </div>
        <div className="max-w-3xl text-sm leading-6 text-muted-foreground md:text-[15px]">
          {t(
            "Read this row before you touch the pool. It surfaces service state, live listener load, the latest ingest pulse, and whether follow-up is waiting in the queue.",
          )}
        </div>
      </div>
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <TopMetricCard
          title={t("Service pulse")}
          value={formatHealthStatus(status, t)}
          description={t("Polled from /healthz every 10 seconds so the shell stays honest.")}
          icon={HeartPulseIcon}
          tone={status === "ok" ? "positive" : "warning"}
        />
        <TopMetricCard
          title={t("Live listeners")}
          value={formatNumber(activeSessions)}
          description={t("Sessions currently consuming the active profile.")}
          icon={RouterIcon}
        />
        <TopMetricCard
          title={t("Pool inventory")}
          value={loadedProxies == null ? "--" : formatNumber(loadedProxies)}
          description={t("Most recent successful subscription load reflected in the runway.")}
          icon={RefreshCwIcon}
        />
        <TopMetricCard
          title={t("Attention queue")}
          value={
            hasWarnings
              ? t("Review")
              : refreshedIps == null
                ? t("Clear")
                : formatNumber(refreshedIps)
          }
          description={
            hasWarnings
              ? t("Warnings are waiting. Review them before opening long-lived listeners.")
              : refreshedIps == null
                ? t("No warnings are queued right now.")
                : t("Latest refresh completed cleanly and updated probe metadata.")
          }
          icon={AlertTriangleIcon}
          tone={hasWarnings ? "warning" : "positive"}
        />
      </div>
    </section>
  );
}
