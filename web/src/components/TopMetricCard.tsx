import type { LucideIcon } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
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
        "overflow-hidden border border-border/70 bg-card/95 shadow-sm transition-colors",
        tone === "positive" && "border-emerald-500/25 bg-emerald-500/[0.06]",
        tone === "warning" && "border-amber-500/25 bg-amber-500/[0.06]",
      )}
    >
      <div
        className={cn(
          "h-1 w-full bg-border/70",
          tone === "positive" && "bg-emerald-500/70",
          tone === "warning" && "bg-amber-500/70",
        )}
      />
      <CardHeader className="gap-5 pb-3">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-3">
            <CardDescription className="text-[0.68rem] font-semibold uppercase tracking-[0.34em] text-muted-foreground/90">
              {title}
            </CardDescription>
            <CardTitle className="text-4xl font-semibold tracking-tight text-foreground">
              {value}
            </CardTitle>
          </div>
          <div className="rounded-2xl border border-border/70 bg-background/90 p-3 shadow-sm">
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
