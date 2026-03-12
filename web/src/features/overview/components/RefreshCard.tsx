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
    <Card className="border-border/70 bg-card/90">
      <CardHeader className="border-b border-border/70">
        <CardTitle className="flex items-center gap-2 text-lg">
          <RefreshCwIcon className="size-4 text-primary" />
          Refresh probes + geo
        </CardTitle>
        <CardDescription>
          Re-run network probes and geo lookup to update the selection pool.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-5 pt-5">
        <form
          className="space-y-4"
          onSubmit={form.handleSubmit(async (values) => {
            await onSubmit(values);
          })}
        >
          <Controller
            control={form.control}
            name="force"
            render={({ field }) => (
              <div className="flex items-start gap-3 rounded-xl border border-border/70 bg-muted/40 p-3">
                <Checkbox
                  checked={field.value}
                  onCheckedChange={(checked) => field.onChange(Boolean(checked))}
                  id="force-refresh"
                />
                <div className="space-y-1">
                  <Label htmlFor="force-refresh">Force refresh stale entries</Label>
                  <p className="text-xs text-muted-foreground">
                    Ignore existing cache hints and attempt to refresh every matching IP.
                  </p>
                </div>
              </div>
            )}
          />
          <div className="flex justify-end">
            <Button disabled={isPending} type="submit" variant="secondary">
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
