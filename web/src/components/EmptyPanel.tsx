import type { LucideIcon } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface EmptyPanelProps {
  title: string;
  description: string;
  icon: LucideIcon;
}

export function EmptyPanel({ title, description, icon: Icon }: EmptyPanelProps) {
  return (
    <Card className="border border-dashed border-border/80 bg-background/80">
      <CardHeader className="items-start gap-3">
        <div className="rounded-full border border-border/80 bg-muted p-2">
          <Icon className="size-4 text-muted-foreground" />
        </div>
        <div>
          <CardTitle>{title}</CardTitle>
          <CardDescription className="mt-1">{description}</CardDescription>
        </div>
      </CardHeader>
      <CardContent className="pt-0 text-sm text-muted-foreground">
        Fill in the form on this page to populate this area.
      </CardContent>
    </Card>
  );
}
