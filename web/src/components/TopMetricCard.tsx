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
        "border border-border/70 bg-card/85 backdrop-blur-sm",
        tone === "positive" && "border-emerald-500/20 bg-emerald-500/5",
        tone === "warning" && "border-amber-500/20 bg-amber-500/5",
      )}
    >
      <CardHeader className="gap-2 border-b border-border/60 pb-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <CardDescription className="uppercase tracking-[0.24em]">{title}</CardDescription>
            <CardTitle className="mt-2 text-3xl font-semibold">{value}</CardTitle>
          </div>
          <div className="rounded-full border border-border/70 bg-background/70 p-2">
            <Icon className="size-4 text-primary" />
          </div>
        </div>
      </CardHeader>
      <CardContent className="pt-4 text-sm text-muted-foreground">{description}</CardContent>
    </Card>
  );
}
