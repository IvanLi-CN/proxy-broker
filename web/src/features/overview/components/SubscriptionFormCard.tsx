import { zodResolver } from "@hookform/resolvers/zod";
import { CableIcon, FileJsonIcon, Link2Icon } from "lucide-react";
import { useEffect } from "react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { Badge } from "@/components/ui/badge";
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
export type SubscriptionFormValues = FormValues;

export const DEFAULT_SUBSCRIPTION_FORM_VALUES: SubscriptionFormValues = {
  sourceType: "url",
  sourceValue: "https://example.com/subscription.yaml",
};

interface SubscriptionFormCardProps {
  isPending: boolean;
  response?: LoadSubscriptionResponse | null;
  error?: string | null;
  onSubmit: (payload: LoadSubscriptionRequest) => void | Promise<void>;
  initialValues?: SubscriptionFormValues;
  onValuesChange?: (values: SubscriptionFormValues) => void;
}

export function SubscriptionFormCard({
  isPending,
  response,
  error,
  onSubmit,
  initialValues = DEFAULT_SUBSCRIPTION_FORM_VALUES,
  onValuesChange,
}: SubscriptionFormCardProps) {
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: initialValues,
  });
  const sourceType = form.watch("sourceType");

  useEffect(() => {
    if (JSON.stringify(form.getValues()) !== JSON.stringify(initialValues)) {
      form.reset(initialValues);
    }
  }, [form, initialValues]);

  useEffect(() => {
    if (!onValuesChange) {
      return;
    }
    const subscription = form.watch((values) => {
      onValuesChange(values as SubscriptionFormValues);
    });
    return () => subscription.unsubscribe();
  }, [form, onValuesChange]);

  return (
    <Card className="overflow-hidden border-border/70 bg-card/96 shadow-[0_24px_70px_-44px_rgba(15,23,42,0.55)]">
      <CardHeader className="gap-4 border-b border-border/70 bg-muted/15 pb-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-2">
            <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
              Primary action
            </div>
            <CardTitle className="flex items-center gap-2 text-xl tracking-tight md:text-2xl">
              <CableIcon className="size-5 text-primary" />
              Load a fresh subscription feed
            </CardTitle>
            <CardDescription className="max-w-2xl text-sm leading-6 text-muted-foreground md:text-[15px]">
              Reset the working inventory for the current profile before you extract IPs or open any
              new listeners.
            </CardDescription>
          </div>
          <div className="flex flex-wrap gap-2">
            <Badge
              variant="outline"
              className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
            >
              source {sourceType}
            </Badge>
            <Badge
              variant="outline"
              className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
            >
              pool reset on success
            </Badge>
          </div>
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
          <div className="grid gap-4 rounded-[28px] border border-border/70 bg-background/80 p-4 md:grid-cols-[minmax(280px,0.34fr)_minmax(0,1fr)]">
            <div className="space-y-2">
              <Label htmlFor="source-type">Source type</Label>
              <Controller
                control={form.control}
                name="sourceType"
                render={({ field }) => (
                  <Select onValueChange={field.onChange} value={field.value}>
                    <SelectTrigger id="source-type" size="lg" className="w-full bg-card">
                      <SelectValue placeholder="Choose source type" />
                    </SelectTrigger>
                    <SelectContent size="lg">
                      <SelectItem size="lg" value="url">
                        <span className="flex items-center gap-2">
                          <Link2Icon className="size-4" /> URL
                        </span>
                      </SelectItem>
                      <SelectItem size="lg" value="file">
                        <span className="flex items-center gap-2">
                          <FileJsonIcon className="size-4" /> File path
                        </span>
                      </SelectItem>
                    </SelectContent>
                  </Select>
                )}
              />
              <p className="min-h-12 text-xs leading-5 text-muted-foreground">
                URL mode fetches remotely; file mode resolves from the Rust host filesystem.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="source-value">Value</Label>
              <Input
                id="source-value"
                size="lg"
                {...form.register("sourceValue")}
                placeholder="https://example.com/subscription.yaml"
                className="bg-card font-mono text-xs md:text-sm"
              />
              {form.formState.errors.sourceValue ? (
                <p className="min-h-12 text-xs text-destructive" role="alert">
                  {form.formState.errors.sourceValue.message}
                </p>
              ) : (
                <p className="min-h-12 text-xs leading-5 text-muted-foreground">
                  {sourceType === "url"
                    ? "Use the upstream subscription URL that the backend can fetch directly."
                    : "Provide a server-local path that the Rust process can read on disk."}
                </p>
              )}
            </div>
          </div>
          <div className="grid gap-3 rounded-[28px] border border-border/70 bg-[linear-gradient(135deg,rgba(59,130,246,0.08),rgba(20,184,166,0.06))] p-4 md:grid-cols-[1fr_auto] md:items-center">
            <div className="space-y-1">
              <div className="text-sm font-semibold text-foreground">What happens next</div>
              <p className="max-w-2xl text-sm leading-6 text-muted-foreground">
                A successful load replaces the candidate pool for this profile. Review warnings at
                once if the upstream feed contains skipped or malformed records.
              </p>
            </div>
            <Button disabled={isPending} size="lg" type="submit" className="min-w-52">
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
