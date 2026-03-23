import {
  ActivityIcon,
  CircleAlertIcon,
  Clock3Icon,
  LoaderCircleIcon,
  RadioTowerIcon,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { formatTimestamp } from "@/lib/format";
import type { TaskSummary } from "@/lib/types";

interface TaskSummaryCardsProps {
  summary: TaskSummary;
  streamState: "connecting" | "live" | "reconnecting";
}

const streamTone = {
  connecting: "border-amber-500/20 bg-amber-500/[0.09] text-amber-700 dark:text-amber-300",
  live: "border-emerald-500/20 bg-emerald-500/[0.09] text-emerald-700 dark:text-emerald-300",
  reconnecting: "border-amber-500/20 bg-amber-500/[0.09] text-amber-700 dark:text-amber-300",
} as const;

export function TaskSummaryCards({ summary, streamState }: TaskSummaryCardsProps) {
  const cards = [
    {
      title: "Live stream",
      value: streamState,
      meta: `Last run ${formatTimestamp(summary.last_run_at)}`,
      icon: RadioTowerIcon,
      accent: streamTone[streamState],
    },
    {
      title: "Running now",
      value: `${summary.running_runs}`,
      meta: `${summary.queued_runs} queued behind`,
      icon: LoaderCircleIcon,
      accent: "border-sky-500/20 bg-sky-500/[0.08] text-sky-700 dark:text-sky-300",
    },
    {
      title: "Failures",
      value: `${summary.failed_runs}`,
      meta: `${summary.skipped_runs} skipped`,
      icon: CircleAlertIcon,
      accent: "border-rose-500/20 bg-rose-500/[0.08] text-rose-700 dark:text-rose-300",
    },
    {
      title: "Succeeded",
      value: `${summary.succeeded_runs}`,
      meta: `${summary.total_runs} total tracked`,
      icon: ActivityIcon,
      accent: "border-emerald-500/20 bg-emerald-500/[0.09] text-emerald-700 dark:text-emerald-300",
    },
    {
      title: "Most recent",
      value: formatTimestamp(summary.last_run_at),
      meta: "Derived from finish/start/create timestamps",
      icon: Clock3Icon,
      accent: "border-violet-500/20 bg-violet-500/[0.08] text-violet-700 dark:text-violet-300",
    },
  ];

  return (
    <section className="grid gap-4 md:grid-cols-2 2xl:grid-cols-5">
      {cards.map((card) => (
        <Card
          key={card.title}
          className="border-border/70 bg-card/96 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]"
        >
          <CardHeader className="flex flex-row items-center justify-between gap-3 pb-3">
            <div className="space-y-1">
              <div className="text-[11px] font-semibold uppercase tracking-[0.28em] text-primary/80">
                {card.title}
              </div>
              <CardTitle className="text-2xl tracking-tight">{card.value}</CardTitle>
            </div>
            <Badge
              variant="outline"
              className={`rounded-full px-3 py-1 text-[11px] ${card.accent}`}
            >
              <card.icon className="mr-1 size-3.5" />
              {card.title}
            </Badge>
          </CardHeader>
          <CardContent className="pt-0 text-sm leading-6 text-muted-foreground">
            {card.meta}
          </CardContent>
        </Card>
      ))}
    </section>
  );
}
