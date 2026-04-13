import { zodResolver } from "@hookform/resolvers/zod";
import { FolderSyncIcon } from "lucide-react";
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

const loadCardSchema = z.object({
  sourceType: z.enum(["url", "file"]),
  sourceValue: z.string().trim().min(1, "validation.source_value_required"),
});

type LoadCardFormValues = z.infer<typeof loadCardSchema>;

interface ProxyLoadCardProps {
  eyebrow: string;
  title: string;
  description: string;
  scopeChip: string;
  pending: boolean;
  response?: LoadSubscriptionResponse | null;
  error?: string | null;
  defaultValue: string;
  submitLabel: string;
  successTitle: string;
  successDescription: string;
  onSubmit: (payload: LoadSubscriptionRequest) => void | Promise<void>;
}

export function ProxyLoadCard({
  eyebrow,
  title,
  description,
  scopeChip,
  pending,
  response,
  error,
  defaultValue,
  submitLabel,
  successTitle,
  successDescription,
  onSubmit,
}: ProxyLoadCardProps) {
  const { t } = useI18n();
  const form = useForm<LoadCardFormValues>({
    resolver: zodResolver(loadCardSchema),
    defaultValues: {
      sourceType: "url",
      sourceValue: defaultValue,
    },
  });
  const sourceType = form.watch("sourceType");

  return (
    <Card className="overflow-hidden border-border/70 bg-card/96 shadow-[0_20px_60px_-40px_rgba(15,23,42,0.5)]">
      <CardHeader className="gap-3 border-b border-border/70 bg-muted/15 pb-4">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div className="space-y-1.5">
            <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
              {eyebrow}
            </div>
            <CardTitle className="flex items-center gap-2 text-lg tracking-tight md:text-xl">
              <FolderSyncIcon className="size-4.5 text-primary" />
              {title}
            </CardTitle>
            <CardDescription className="max-w-xl text-sm leading-5 text-muted-foreground">
              {description}
            </CardDescription>
          </div>
          <div className="flex flex-wrap gap-1.5">
            <Badge
              variant="outline"
              className="rounded-full px-2.5 py-0.5 font-mono text-[10px] uppercase tracking-[0.16em]"
            >
              {scopeChip}
            </Badge>
            <Badge
              variant="outline"
              className="rounded-full px-2.5 py-0.5 font-mono text-[10px] uppercase tracking-[0.16em]"
            >
              {sourceType === "url" ? t("remote fetch") : t("host file")}
            </Badge>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4 pt-4">
        <form
          className="space-y-3"
          onSubmit={form.handleSubmit((values) =>
            onSubmit({
              source: {
                type: values.sourceType,
                value: values.sourceValue.trim(),
              },
            }),
          )}
        >
          <div className="grid gap-3 rounded-[20px] border border-border/70 bg-background/80 p-3 md:grid-cols-[168px_minmax(0,1fr)]">
            <div className="space-y-2">
              <Label htmlFor={`${eyebrow}-source-type`}>{t("Source type")}</Label>
              <Controller
                control={form.control}
                name="sourceType"
                render={({ field }) => (
                  <Select onValueChange={field.onChange} value={field.value}>
                    <SelectTrigger
                      id={`${eyebrow}-source-type`}
                      size="lg"
                      className="w-full bg-card"
                    >
                      <SelectValue placeholder={t("Choose source type")} />
                    </SelectTrigger>
                    <SelectContent size="lg">
                      <SelectItem size="lg" value="url">
                        {t("URL")}
                      </SelectItem>
                      <SelectItem size="lg" value="file">
                        {t("File path")}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor={`${eyebrow}-source-value`}>{t("Value")}</Label>
              <Input
                id={`${eyebrow}-source-value`}
                size="lg"
                {...form.register("sourceValue")}
                placeholder="https://example.com/subscription.yaml"
                className="bg-card font-mono text-xs md:text-sm"
              />
            </div>
          </div>

          <div className="flex justify-end">
            <Button disabled={pending} size="lg" type="submit" className="min-w-40">
              {pending ? t("Loading subscription...") : submitLabel}
            </Button>
          </div>

          {form.formState.errors.sourceValue ? (
            <p className="text-xs text-destructive" role="alert">
              {t(form.formState.errors.sourceValue.message ?? "validation.source_value_required")}
            </p>
          ) : (
            <div className="flex flex-wrap gap-x-3 gap-y-1 text-xs leading-5 text-muted-foreground">
              <span>
                {sourceType === "url"
                  ? t("Use the upstream subscription URL that the backend can fetch directly.")
                  : t("Provide a server-local path that the Rust process can read on disk.")}
              </span>
              <span className="hidden text-border md:inline">•</span>
              <span>{t("Re-import restores nodes that still exist upstream.")}</span>
            </div>
          )}
        </form>

        {response ? (
          <ActionResponsePanel
            title={successTitle}
            description={successDescription}
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
