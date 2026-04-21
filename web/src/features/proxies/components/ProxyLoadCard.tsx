import { zodResolver } from "@hookform/resolvers/zod";
import { FolderSyncIcon, Link2Icon, ServerCogIcon } from "lucide-react";
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
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { useI18n } from "@/i18n";
import { buildSubscriptionMetadataBullets } from "@/lib/format";
import type { LoadSubscriptionRequest, LoadSubscriptionResponse } from "@/lib/types";

const loadCardSchema = z
  .object({
    importMode: z.enum(["subscription", "nodes"]),
    name: z.string().default(""),
    sourceType: z.enum(["url", "file"]),
    sourceValue: z.string().default(""),
    content: z.string().default(""),
  })
  .superRefine((values, ctx) => {
    if (values.importMode === "subscription" && values.sourceValue.trim().length === 0) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["sourceValue"],
        message: "validation.source_value_required",
      });
    }
    if (values.importMode === "nodes" && values.content.trim().length === 0) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["content"],
        message: "validation.content_required",
      });
    }
  });

type LoadCardFormValues = z.input<typeof loadCardSchema>;
type LoadCardSubmitValues = z.output<typeof loadCardSchema>;

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
  const { locale, t } = useI18n();
  const form = useForm<LoadCardFormValues, undefined, LoadCardSubmitValues>({
    resolver: zodResolver(loadCardSchema),
    defaultValues: {
      importMode: "subscription",
      name: "",
      sourceType: "url",
      sourceValue: defaultValue,
      content: "",
    },
  });
  const importMode = form.watch("importMode");
  const sourceType = form.watch("sourceType");
  const responseBullets = response
    ? buildSubscriptionMetadataBullets(locale, t, response)
    : undefined;

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
              {importMode === "subscription" ? t("subscription source") : t("node group")}
            </Badge>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4 pt-4">
        <form
          className="space-y-3"
          onSubmit={form.handleSubmit((values) => {
            const explicitName = values.name.trim();
            const payload: LoadSubscriptionRequest =
              values.importMode === "subscription"
                ? {
                    name: explicitName || undefined,
                    source: {
                      type: values.sourceType,
                      value: values.sourceValue.trim(),
                    },
                  }
                : {
                    name: explicitName || undefined,
                    content: values.content.trim(),
                  };
            return onSubmit(payload);
          })}
        >
          <div className="space-y-3 rounded-[20px] border border-border/70 bg-background/80 p-3">
            <div className="space-y-2">
              <Label>{t("Import type")}</Label>
              <Controller
                control={form.control}
                name="importMode"
                render={({ field }) => (
                  <Tabs value={field.value} onValueChange={field.onChange} className="w-full">
                    <TabsList className="w-full justify-start">
                      <TabsTrigger value="subscription">
                        <Link2Icon className="size-4" />
                        {t("Subscription")}
                      </TabsTrigger>
                      <TabsTrigger value="nodes">
                        <ServerCogIcon className="size-4" />
                        {t("Nodes")}
                      </TabsTrigger>
                    </TabsList>
                  </Tabs>
                )}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor={`${eyebrow}-name`}>{t("Name")}</Label>
              <Input
                id={`${eyebrow}-name`}
                size="lg"
                {...form.register("name")}
                placeholder={
                  importMode === "subscription"
                    ? t("Leave blank to parse the upstream subscription title automatically")
                    : t("Leave blank to group nodes by the first proxy name")
                }
                className="bg-card text-sm"
              />
              <p className="text-xs leading-5 text-muted-foreground">
                {importMode === "subscription"
                  ? t(
                      "Optional. Leave blank to use the parsed subscription title when available; otherwise the list falls back to the saved name or import ID.",
                    )
                  : t(
                      "Optional. Leave blank to auto-name the node group from its first proxy; if that is unavailable, the list falls back to the import ID.",
                    )}
              </p>
            </div>

            {importMode === "subscription" ? (
              <div className="grid gap-3 md:grid-cols-[168px_minmax(0,1fr)]">
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
            ) : (
              <div className="space-y-2">
                <Label htmlFor={`${eyebrow}-node-content`}>{t("Nodes content")}</Label>
                <Textarea
                  id={`${eyebrow}-node-content`}
                  size="lg"
                  {...form.register("content")}
                  placeholder={
                    "proxies:\n  - name: hk-entry\n    type: socks5\n    server: 203.0.113.10\n    port: 1080"
                  }
                  className="min-h-44 bg-card font-mono text-xs md:text-sm"
                />
                <p className="text-xs leading-5 text-muted-foreground">
                  {t(
                    "Paste one or more Clash-compatible nodes as `proxies:` YAML or a plain list. Everything in the textarea is imported as one original node group.",
                  )}
                </p>
              </div>
            )}
          </div>

          <div className="flex justify-end">
            <Button disabled={pending} size="lg" type="submit" className="min-w-40">
              {pending ? t("Loading subscription...") : submitLabel}
            </Button>
          </div>

          {importMode === "subscription" && form.formState.errors.sourceValue ? (
            <p className="text-xs text-destructive" role="alert">
              {t(form.formState.errors.sourceValue.message ?? "validation.source_value_required")}
            </p>
          ) : null}
          {importMode === "nodes" && form.formState.errors.content ? (
            <p className="text-xs text-destructive" role="alert">
              {t(form.formState.errors.content.message ?? "validation.content_required")}
            </p>
          ) : null}
          {!form.formState.errors.sourceValue && !form.formState.errors.content ? (
            <div className="flex flex-wrap gap-x-3 gap-y-1 text-xs leading-5 text-muted-foreground">
              <span>
                {importMode === "subscription"
                  ? sourceType === "url"
                    ? t("Use the upstream subscription URL that the backend can fetch directly.")
                    : t("Provide a server-local path that the Rust process can read on disk.")
                  : t(
                      "Each submit creates one original import group that can later be reallocated or deleted as a whole.",
                    )}
              </span>
              <span className="hidden text-border md:inline">•</span>
              <span>
                {importMode === "subscription"
                  ? t("Re-import restores nodes that still exist upstream.")
                  : t(
                      "Batch node imports keep every pasted node inside the same allocation group.",
                    )}
              </span>
            </div>
          ) : null}
        </form>

        {response ? (
          <ActionResponsePanel
            title={successTitle}
            description={
              response.resolved_name
                ? t("{description} Final name: {name}.", {
                    description: successDescription,
                    name: response.resolved_name,
                  })
                : successDescription
            }
            tone={response.warnings.length > 0 ? "warning" : "success"}
            bullets={responseBullets}
          />
        ) : null}
        {error ? (
          <ActionResponsePanel title={t("Load failed")} description={error} tone="error" />
        ) : null}
      </CardContent>
    </Card>
  );
}
