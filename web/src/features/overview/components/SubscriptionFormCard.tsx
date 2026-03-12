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
    <Card className="border-border/70 bg-card/90">
      <CardHeader className="border-b border-border/70">
        <CardTitle className="flex items-center gap-2 text-lg">
          <CableIcon className="size-4 text-primary" />
          Load subscription
        </CardTitle>
        <CardDescription>
          Pull a mihomo subscription from a URL or point the service at a local file path.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-5 pt-5">
        <form
          className="grid gap-4"
          onSubmit={form.handleSubmit((values) =>
            onSubmit({
              source: {
                type: values.sourceType,
                value: values.sourceValue.trim(),
              },
            }),
          )}
        >
          <div className="grid gap-4 md:grid-cols-[180px_1fr]">
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
                <p className="text-xs text-muted-foreground">
                  For file mode, enter the path as seen by the Rust service process.
                </p>
              )}
            </div>
          </div>
          <div className="flex justify-end">
            <Button disabled={isPending} type="submit">
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
