import { zodResolver } from "@hookform/resolvers/zod";
import { CableIcon, FileJsonIcon, Link2Icon } from "lucide-react";
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
import { useI18n } from "@/i18n";
import { formatOperatorWarning } from "@/lib/format";
import type { LoadSubscriptionRequest, LoadSubscriptionResponse } from "@/lib/types";

const schema = z.object({
  sourceType: z.enum(["url", "file"]),
  sourceValue: z.string().trim().min(1, "validation.source_value_required"),
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
  const { t } = useI18n();
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      sourceType: "url",
      sourceValue: "https://example.com/subscription.yaml",
    },
  });
  const sourceType = form.watch("sourceType");
  const sourceTypeLabel = sourceType === "url" ? t("URL") : t("File path");

  return (
    <Card className="overflow-hidden border-border/70 bg-card/96 shadow-[0_24px_70px_-44px_rgba(15,23,42,0.55)]">
      <CardHeader className="gap-4 border-b border-border/70 bg-muted/15 pb-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-2">
            <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
              {t("Primary action")}
            </div>
            <CardTitle className="flex items-center gap-2 text-xl tracking-tight md:text-2xl">
              <CableIcon className="size-5 text-primary" />
              {t("Load a fresh subscription feed")}
            </CardTitle>
            <CardDescription className="max-w-2xl text-sm leading-6 text-muted-foreground md:text-[15px]">
              {t(
                "Reset the working inventory for the current profile before you extract IPs or open any new listeners.",
              )}
            </CardDescription>
          </div>
          <div className="flex flex-wrap gap-2">
            <Badge
              variant="outline"
              className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
            >
              {t("source {sourceType}", { sourceType: sourceTypeLabel })}
            </Badge>
            <Badge
              variant="outline"
              className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
            >
              {t("pool reset on success")}
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
              <Label htmlFor="source-type">{t("Source type")}</Label>
              <Controller
                control={form.control}
                name="sourceType"
                render={({ field }) => (
                  <Select onValueChange={field.onChange} value={field.value}>
                    <SelectTrigger id="source-type" size="lg" className="w-full bg-card">
                      <SelectValue placeholder={t("Choose source type")} />
                    </SelectTrigger>
                    <SelectContent size="lg">
                      <SelectItem size="lg" value="url">
                        <span className="flex items-center gap-2">
                          <Link2Icon className="size-4" /> {t("URL")}
                        </span>
                      </SelectItem>
                      <SelectItem size="lg" value="file">
                        <span className="flex items-center gap-2">
                          <FileJsonIcon className="size-4" /> {t("File path")}
                        </span>
                      </SelectItem>
                    </SelectContent>
                  </Select>
                )}
              />
              <p className="min-h-12 text-xs leading-5 text-muted-foreground">
                {t("URL mode fetches remotely; file mode resolves from the Rust host filesystem.")}
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="source-value">{t("Value")}</Label>
              <Input
                id="source-value"
                size="lg"
                {...form.register("sourceValue")}
                placeholder="https://example.com/subscription.yaml"
                className="bg-card font-mono text-xs md:text-sm"
              />
              {form.formState.errors.sourceValue ? (
                <p className="min-h-12 text-xs text-destructive" role="alert">
                  {t(
                    form.formState.errors.sourceValue.message ?? "validation.source_value_required",
                  )}
                </p>
              ) : (
                <p className="min-h-12 text-xs leading-5 text-muted-foreground">
                  {sourceType === "url"
                    ? t("Use the upstream subscription URL that the backend can fetch directly.")
                    : t("Provide a server-local path that the Rust process can read on disk.")}
                </p>
              )}
            </div>
          </div>
          <div className="grid gap-3 rounded-[28px] border border-border/70 bg-[linear-gradient(135deg,rgba(59,130,246,0.08),rgba(20,184,166,0.06))] p-4 md:grid-cols-[1fr_auto] md:items-center">
            <div className="space-y-1">
              <div className="text-sm font-semibold text-foreground">{t("What happens next")}</div>
              <p className="max-w-2xl text-sm leading-6 text-muted-foreground">
                {t(
                  "A successful load replaces the candidate pool for this profile. Review warnings at once if the upstream feed contains skipped or malformed records.",
                )}
              </p>
            </div>
            <Button disabled={isPending} size="lg" type="submit" className="min-w-52">
              {isPending ? t("Loading subscription...") : t("Load subscription")}
            </Button>
          </div>
        </form>
        {response ? (
          <ActionResponsePanel
            title={t("Subscription loaded")}
            description={t("Loaded {proxyCount} proxies across {ipCount} distinct IPs.", {
              proxyCount: response.loaded_proxies,
              ipCount: response.distinct_ips,
            })}
            tone={response.warnings.length > 0 ? "warning" : "success"}
            bullets={response.warnings.map((warning) => formatOperatorWarning(t, warning))}
          />
        ) : null}
        {error ? (
          <ActionResponsePanel title={t("Load failed")} description={error} tone="error" />
        ) : null}
      </CardContent>
    </Card>
  );
}
