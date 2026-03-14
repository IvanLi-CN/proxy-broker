import type { LucideIcon } from "lucide-react";

import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface TopMetricCardProps {
  title: string;
  value: string;
  description: string;
  icon: LucideIcon;
  tone?: "default" | "positive" | "warning";
}

export function TopMetricCard({
  title,
  value,
  description,
  icon: Icon,
  tone = "default",
}: TopMetricCardProps) {
  return (
    <Card
      className={cn(
        "overflow-hidden border-border/70 bg-card/96 shadow-[0_18px_48px_-36px_rgba(15,23,42,0.45)] transition-transform duration-200 hover:-translate-y-0.5",
        tone === "positive" && "border-emerald-500/22 bg-emerald-500/[0.07]",
        tone === "warning" && "border-amber-500/24 bg-amber-500/[0.08]",
      )}
    >
      <CardHeader className="gap-4 pb-0">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-2">
            <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-muted-foreground">
              {title}
            </div>
            <div className="text-3xl font-semibold tracking-[-0.04em] text-foreground md:text-4xl">
              {value}
            </div>
          </div>
          <div
            className={cn(
              "rounded-2xl border border-border/70 bg-background/80 p-3 shadow-sm",
              tone === "positive" && "border-emerald-500/18",
              tone === "warning" && "border-amber-500/18",
            )}
          >
            <Icon className="size-4 text-primary" />
          </div>
        </div>
      </CardHeader>
      <CardContent className="pt-0 text-sm leading-6 text-muted-foreground">
        {description}
      </CardContent>
    </Card>
  );
}
