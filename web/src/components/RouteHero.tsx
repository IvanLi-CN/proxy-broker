import type { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface RouteHeroBadge {
  label: string;
  tone?: "default" | "positive" | "warning" | "danger" | "neutral";
}

interface RouteHeroProps {
  eyebrow: string;
  title: string;
  description: string;
  badges?: RouteHeroBadge[];
  actions?: ReactNode;
  aside?: ReactNode;
}

const badgeStyles: Record<NonNullable<RouteHeroBadge["tone"]>, string> = {
  default: "border-primary/20 bg-primary/10 text-primary",
  positive: "border-emerald-500/25 bg-emerald-500/12 text-emerald-700 dark:text-emerald-300",
  warning: "border-amber-500/25 bg-amber-500/14 text-amber-800 dark:text-amber-300",
  danger: "border-destructive/25 bg-destructive/10 text-destructive",
  neutral: "border-foreground/10 bg-background/80 text-muted-foreground",
};

export function RouteHero({
  eyebrow,
  title,
  description,
  badges = [],
  actions,
  aside,
}: RouteHeroProps) {
  return (
    <section className="relative overflow-hidden rounded-[32px] border border-border/70 bg-[linear-gradient(140deg,rgba(255,255,255,0.94),rgba(238,244,255,0.96)_48%,rgba(232,242,242,0.92))] p-6 shadow-[0_24px_80px_-36px_rgba(15,23,42,0.45)] dark:bg-[linear-gradient(140deg,rgba(15,23,42,0.94),rgba(17,24,39,0.96)_48%,rgba(15,40,48,0.9))] md:p-8">
      <div className="pointer-events-none absolute inset-y-0 right-0 hidden w-1/3 bg-[radial-gradient(circle_at_top,rgba(59,130,246,0.18),transparent_55%),radial-gradient(circle_at_bottom,rgba(20,184,166,0.16),transparent_48%)] lg:block" />
      <div className="relative grid gap-6 xl:grid-cols-[minmax(0,1.25fr)_320px] xl:items-start">
        <div className="space-y-5">
          <div className="space-y-3">
            <div className="text-[11px] font-semibold uppercase tracking-[0.36em] text-primary/80">
              {eyebrow}
            </div>
            <h1 className="max-w-4xl text-3xl font-semibold tracking-[-0.04em] text-foreground md:text-5xl">
              {title}
            </h1>
            <p className="max-w-3xl text-sm leading-7 text-muted-foreground md:text-base">
              {description}
            </p>
          </div>
          {badges.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {badges.map((badge) => (
                <Badge
                  key={`${badge.label}-${badge.tone ?? "default"}`}
                  variant="outline"
                  className={cn(
                    "rounded-full border px-3 py-1 font-mono text-[11px] tracking-[0.18em] uppercase",
                    badgeStyles[badge.tone ?? "default"],
                  )}
                >
                  {badge.label}
                </Badge>
              ))}
            </div>
          ) : null}
          {actions ? <div className="flex flex-wrap gap-3">{actions}</div> : null}
        </div>
        {aside ? (
          <div className="relative rounded-[28px] border border-border/70 bg-background/72 p-4 shadow-[0_12px_48px_-32px_rgba(15,23,42,0.45)] backdrop-blur-sm md:p-5">
            {aside}
          </div>
        ) : null}
      </div>
    </section>
  );
}
