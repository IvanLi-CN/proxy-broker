import { zodResolver } from "@hookform/resolvers/zod";
import { RefreshCwIcon } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { useI18n } from "@/i18n";
import type { RefreshRequest, RefreshResponse } from "@/lib/types";

const schema = z.object({
  force: z.boolean(),
});

type FormValues = z.infer<typeof schema>;

interface RefreshCardProps {
  isPending: boolean;
  response?: RefreshResponse | null;
  error?: string | null;
  onSubmit: (payload: RefreshRequest) => void | Promise<void>;
}

export function RefreshCard({ isPending, response, error, onSubmit }: RefreshCardProps) {
  const { t } = useI18n();
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: { force: false },
  });

  return (
    <Card className="overflow-hidden border-border/70 bg-card/96 shadow-[0_24px_70px_-44px_rgba(15,23,42,0.55)]">
      <CardHeader className="gap-4 border-b border-border/70 bg-muted/15 pb-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-2">
            <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
              {t("Probe refresh")}
            </div>
            <CardTitle className="flex items-center gap-2 text-xl tracking-tight">
              <RefreshCwIcon className="size-5 text-primary" />
              {t("Refresh probes and geo hints")}
            </CardTitle>
            <CardDescription className="text-sm leading-6 text-muted-foreground md:text-[15px]">
              {t(
                "Use this when latency or geo attribution feels stale. The refresh updates operator hints without changing profile identity.",
              )}
            </CardDescription>
          </div>
          <Badge
            variant="outline"
            className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
          >
            {t("safe to repeat")}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-6 pt-6">
        <form
          className="space-y-6"
          onSubmit={form.handleSubmit(async (values) => {
            await onSubmit(values);
          })}
        >
          <Controller
            control={form.control}
            name="force"
            render={({ field }) => (
              <div className="grid gap-4 rounded-[28px] border border-border/70 bg-background/80 p-4 md:grid-cols-[auto_1fr]">
                <Checkbox
                  checked={field.value}
                  onCheckedChange={(checked) => field.onChange(Boolean(checked))}
                  id="force-refresh"
                  className="mt-1"
                />
                <div className="space-y-1.5">
                  <Label htmlFor="force-refresh">{t("Force refresh stale entries")}</Label>
                  <p className="text-sm leading-6 text-muted-foreground">
                    {t(
                      "Ignore cached probe hints and attempt a full refresh for every matching IP when the current metadata looks suspiciously old.",
                    )}
                  </p>
                </div>
              </div>
            )}
          />
          <div className="grid gap-3 rounded-[28px] border border-border/70 bg-[linear-gradient(135deg,rgba(59,130,246,0.08),rgba(245,158,11,0.08))] p-4 md:grid-cols-[1fr_auto] md:items-center">
            <div className="space-y-1">
              <div className="text-sm font-semibold text-foreground">{t("Operator hint")}</div>
              <p className="max-w-xl text-sm leading-6 text-muted-foreground">
                {t(
                  "If extracts suddenly return empty or geo labels look wrong, run this once before assuming the feed itself is bad.",
                )}
              </p>
            </div>
            <Button
              disabled={isPending}
              size="lg"
              type="submit"
              variant="secondary"
              className="min-w-48"
            >
              {isPending ? t("Refreshing...") : t("Refresh metadata")}
            </Button>
          </div>
        </form>
        {response ? (
          <ActionResponsePanel
            title={t("Refresh completed")}
            description={t(
              "Probed {probed} IPs, updated {geoUpdated} geo records, skipped {skipped} cached entries.",
              {
                probed: response.probed_ips,
                geoUpdated: response.geo_updated,
                skipped: response.skipped_cached,
              },
            )}
          />
        ) : null}
        {error ? (
          <ActionResponsePanel title={t("Refresh failed")} description={error} tone="error" />
        ) : null}
      </CardContent>
    </Card>
  );
}
