import { zodResolver } from "@hookform/resolvers/zod";
import { CableIcon, FileJsonIcon, Link2Icon } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { LoadSubscriptionRequest, LoadSubscriptionResponse } from "@/lib/types";

const schema = z.object({
  sourceType: z.enum(["url", "file"]),
  sourceValue: z.string().trim().min(1, "Source value is required"),
});

type FormValues = z.infer<typeof schema>;

interface SubscriptionFormCardProps {
  isPending: boolean;
  response?: LoadSubscriptionResponse | null;
  error?: string | null;
  onSubmit: (payload: LoadSubscriptionRequest) => void | Promise<void>;
}

export function SubscriptionFormCard({
  isPending,
  response,
  error,
  onSubmit,
}: SubscriptionFormCardProps) {
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      sourceType: "url",
      sourceValue: "https://example.com/subscription.yaml",
    },
  });

  return (
    <Card className="overflow-hidden border-border/70 bg-card/95 shadow-sm">
      <CardHeader className="space-y-3 border-b border-border/70 bg-muted/20 pb-5">
        <div className="text-xs font-semibold uppercase tracking-[0.3em] text-primary/80">
          Primary action
        </div>
        <div className="space-y-2">
          <CardTitle className="flex items-center gap-2 text-xl tracking-tight md:text-2xl">
            <CableIcon className="size-5 text-primary" />
            Load a fresh subscription feed
          </CardTitle>
          <CardDescription className="max-w-2xl text-sm leading-6 text-muted-foreground md:text-[15px]">
            Start every operator run here. Pull a mihomo subscription from a URL, or point the Rust
            service at a local file path on the host to refresh the pool before you inspect IPs or
            open listeners.
          </CardDescription>
        </div>
      </CardHeader>
      <CardContent className="space-y-6 pt-6">
        <form
          className="space-y-6"
          onSubmit={form.handleSubmit((values) =>
            onSubmit({
              source: {
                type: values.sourceType,
                value: values.sourceValue.trim(),
              },
            }),
          )}
        >
          <div className="grid gap-4 rounded-2xl border border-border/70 bg-background/80 p-4 md:grid-cols-[200px_1fr]">
            <div className="space-y-2">
              <Label htmlFor="source-type">Source type</Label>
              <Controller
                control={form.control}
                name="sourceType"
                render={({ field }) => (
                  <Select onValueChange={field.onChange} value={field.value}>
                    <SelectTrigger id="source-type" className="w-full">
                      <SelectValue placeholder="Choose source type" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="url">
                        <span className="flex items-center gap-2">
                          <Link2Icon className="size-4" /> URL
                        </span>
                      </SelectItem>
                      <SelectItem value="file">
                        <span className="flex items-center gap-2">
                          <FileJsonIcon className="size-4" /> File path
                        </span>
                      </SelectItem>
                    </SelectContent>
                  </Select>
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="source-value">Value</Label>
              <Input
                id="source-value"
                {...form.register("sourceValue")}
                placeholder="https://example.com/subscription.yaml"
                className="font-mono text-xs md:text-sm"
              />
              {form.formState.errors.sourceValue ? (
                <p className="text-xs text-destructive">
                  {form.formState.errors.sourceValue.message}
                </p>
              ) : (
                <p className="text-xs leading-5 text-muted-foreground">
                  URL mode fetches over the network; file mode resolves from the Rust service host,
                  not the browser sandbox.
                </p>
              )}
            </div>
          </div>
          <div className="flex flex-col gap-3 rounded-2xl border border-dashed border-border/70 bg-muted/20 px-4 py-4 md:flex-row md:items-center md:justify-between">
            <p className="max-w-xl text-sm leading-6 text-muted-foreground">
              A successful load replaces the candidate pool for the current profile. Review warnings
              immediately if the upstream feed contains skipped or malformed records.
            </p>
            <Button disabled={isPending} size="lg" type="submit" className="min-w-48">
              {isPending ? "Loading subscription..." : "Load subscription"}
            </Button>
          </div>
        </form>
        {response ? (
          <ActionResponsePanel
            title="Subscription loaded"
            description={`Loaded ${response.loaded_proxies} proxies across ${response.distinct_ips} distinct IPs.`}
            tone={response.warnings.length > 0 ? "warning" : "success"}
            bullets={response.warnings}
          />
        ) : null}
        {error ? (
          <ActionResponsePanel title="Load failed" description={error} tone="error" />
        ) : null}
      </CardContent>
    </Card>
  );
}
