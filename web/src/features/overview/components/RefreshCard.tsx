import { zodResolver } from "@hookform/resolvers/zod";
import { RefreshCwIcon } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
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
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: { force: false },
  });

  return (
    <Card className="overflow-hidden border-border/70 bg-card/95 shadow-sm">
      <CardHeader className="space-y-3 border-b border-border/70 bg-muted/20 pb-5">
        <div className="text-xs font-semibold uppercase tracking-[0.3em] text-primary/80">
          Secondary action
        </div>
        <div className="space-y-2">
          <CardTitle className="flex items-center gap-2 text-xl tracking-tight">
            <RefreshCwIcon className="size-5 text-primary" />
            Refresh probes and geo hints
          </CardTitle>
          <CardDescription className="text-sm leading-6 text-muted-foreground md:text-[15px]">
            Run this after loading a new feed, or whenever upstream latency and geo attribution look
            stale. It updates the selection pool without changing your profile identity.
          </CardDescription>
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
              <div className="flex items-start gap-3 rounded-2xl border border-border/70 bg-background/80 p-4">
                <Checkbox
                  checked={field.value}
                  onCheckedChange={(checked) => field.onChange(Boolean(checked))}
                  id="force-refresh"
                />
                <div className="space-y-1.5">
                  <Label htmlFor="force-refresh">Force refresh stale entries</Label>
                  <p className="text-sm leading-6 text-muted-foreground">
                    Ignore cached probe hints and attempt a full refresh for every matching IP. Use
                    this when upstream geo changed sharply or results look suspiciously old.
                  </p>
                </div>
              </div>
            )}
          />
          <div className="flex flex-col gap-3 rounded-2xl border border-dashed border-border/70 bg-muted/20 px-4 py-4 md:flex-row md:items-center md:justify-between">
            <p className="max-w-xl text-sm leading-6 text-muted-foreground">
              A refresh is safe to repeat. If extraction starts returning empty sets, run this once
              before assuming the subscription itself is bad.
            </p>
            <Button
              disabled={isPending}
              size="lg"
              type="submit"
              variant="secondary"
              className="min-w-44"
            >
              {isPending ? "Refreshing..." : "Refresh metadata"}
            </Button>
          </div>
        </form>
        {response ? (
          <ActionResponsePanel
            title="Refresh completed"
            description={`Probed ${response.probed_ips} IPs, updated ${response.geo_updated} geo records, skipped ${response.skipped_cached} cached entries.`}
          />
        ) : null}
        {error ? (
          <ActionResponsePanel title="Refresh failed" description={error} tone="error" />
        ) : null}
      </CardContent>
    </Card>
  );
}
