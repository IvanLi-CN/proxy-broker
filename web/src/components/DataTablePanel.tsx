import type { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface DataTablePanelProps {
  eyebrow: string;
  title: string;
  description: string;
  chips?: string[];
  actions?: ReactNode;
  children: ReactNode;
}

export function DataTablePanel({
  eyebrow,
  title,
  description,
  chips = [],
  actions,
  children,
}: DataTablePanelProps) {
  return (
    <Card className="overflow-hidden border-border/70 bg-card/94 shadow-[0_20px_60px_-40px_rgba(15,23,42,0.5)]">
      <CardHeader className="gap-4 border-b border-border/70 bg-muted/15 pb-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-2">
            <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
              {eyebrow}
            </div>
            <div className="space-y-1">
              <CardTitle className="text-xl tracking-tight md:text-2xl">{title}</CardTitle>
              <CardDescription className="max-w-3xl text-sm leading-6 text-muted-foreground md:text-[15px]">
                {description}
              </CardDescription>
            </div>
          </div>
          {actions ? <div className="flex shrink-0 flex-wrap gap-2">{actions}</div> : null}
        </div>
        {chips.length > 0 ? (
          <div className="flex flex-wrap gap-2">
            {chips.map((chip) => (
              <Badge
                key={chip}
                variant="outline"
                className="rounded-full bg-background/80 px-3 py-1 text-[11px] text-muted-foreground"
              >
                {chip}
              </Badge>
            ))}
          </div>
        ) : null}
      </CardHeader>
      <CardContent className="pt-5">{children}</CardContent>
    </Card>
  );
}
