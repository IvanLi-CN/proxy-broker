import type { LucideIcon } from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useI18n } from "@/i18n";

interface EmptyPanelProps {
  title: string;
  description: string;
  icon: LucideIcon;
  hint?: string;
}

export function EmptyPanel({ title, description, icon: Icon, hint }: EmptyPanelProps) {
  const { t } = useI18n();

  return (
    <Card className="border border-dashed border-border/80 bg-background/76 shadow-none">
      <CardHeader className="items-start gap-4">
        <div className="rounded-2xl border border-border/80 bg-muted/60 p-3">
          <Icon className="size-4 text-muted-foreground" />
        </div>
        <div className="space-y-1.5">
          <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-muted-foreground">
            {t("Empty state")}
          </div>
          <CardTitle className="text-lg tracking-tight">{title}</CardTitle>
          <p className="text-sm leading-6 text-muted-foreground">{description}</p>
        </div>
      </CardHeader>
      <CardContent className="pt-0 text-sm leading-6 text-muted-foreground">
        {hint ?? t("Fill in the controls on this page to populate this area.")}
      </CardContent>
    </Card>
  );
}
