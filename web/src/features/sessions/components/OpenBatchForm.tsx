import { zodResolver } from "@hookform/resolvers/zod";
import { PlusIcon, Rows4Icon, Trash2Icon } from "lucide-react";
import { Controller, useFieldArray, useForm } from "react-hook-form";
import { z } from "zod";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { StringListField } from "@/components/StringListField";
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
import { buildOpenSessionRequest, formatSortMode } from "@/lib/format";
import type { OpenBatchRequest, OpenBatchResponse, SortMode } from "@/lib/types";

const rowSchema = z.object({
  specifiedIp: z.string(),
  desiredPort: z.string(),
  countryCodes: z.string(),
  cities: z.string(),
  selectorSpecifiedIps: z.string(),
  blacklistIps: z.string(),
  limit: z.string(),
  sortMode: z.enum(["mru", "lru"] satisfies SortMode[]),
});

const schema = z.object({
  requests: z.array(rowSchema).min(1),
});

type BatchRequestRow = z.infer<typeof rowSchema>;
type FormValues = z.infer<typeof schema>;

const emptyRow = (): BatchRequestRow => ({
  specifiedIp: "",
  desiredPort: "",
  countryCodes: "JP",
  cities: "",
  selectorSpecifiedIps: "",
  blacklistIps: "",
  limit: "1",
  sortMode: "lru",
});

interface OpenBatchFormProps {
  isPending: boolean;
  response?: OpenBatchResponse | null;
  error?: string | null;
  onSubmit: (payload: OpenBatchRequest) => void | Promise<void>;
}

export function OpenBatchForm({ isPending, response, error, onSubmit }: OpenBatchFormProps) {
  const { t } = useI18n();
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      requests: [emptyRow(), { ...emptyRow(), desiredPort: "10081" }],
    },
  });
  const fieldArray = useFieldArray({ control: form.control, name: "requests" });

  return (
    <Card className="overflow-hidden border-border/70 bg-card/96 shadow-[0_24px_70px_-44px_rgba(15,23,42,0.55)]">
      <CardHeader className="gap-4 border-b border-border/70 bg-muted/15 pb-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-2">
            <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
              {t("Batch open")}
            </div>
            <CardTitle className="flex items-center gap-2 text-xl tracking-tight">
              <Rows4Icon className="size-4 text-primary" />
              {t("Queue a transactional batch")}
            </CardTitle>
            <CardDescription className="text-sm leading-6 text-muted-foreground md:text-[15px]">
              {t(
                "Stage multiple open-session requests and let the backend roll the whole set back if any row fails validation or allocation.",
              )}
            </CardDescription>
          </div>
          <Badge
            variant="outline"
            className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
          >
            {t("rollback on failure")}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-5 pt-5">
        <form
          className="space-y-4"
          onSubmit={form.handleSubmit(async (values) => {
            await onSubmit({
              requests: values.requests.map((row: BatchRequestRow) => buildOpenSessionRequest(row)),
            });
          })}
        >
          <div className="space-y-4">
            {fieldArray.fields.map((field, index) => (
              <div
                key={field.id}
                className="rounded-[28px] border border-border/70 bg-background/80 p-4"
              >
                <div className="mb-4 flex items-center justify-between gap-3">
                  <div>
                    <div className="text-sm font-semibold text-foreground">
                      {t("Request #{index}", { index: index + 1 })}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {t("Compact selector for one listener entry.")}
                    </div>
                  </div>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    onClick={() => fieldArray.remove(index)}
                    disabled={fieldArray.fields.length === 1}
                    aria-label={t("Remove request {index}", { index: index + 1 })}
                  >
                    <Trash2Icon />
                  </Button>
                </div>
                <div className="grid gap-4">
                  <div className="space-y-2">
                    <Label htmlFor={`batch-specified-ip-${index}`}>{t("Specified IP")}</Label>
                    <Input
                      id={`batch-specified-ip-${index}`}
                      {...form.register(`requests.${index}.specifiedIp`)}
                      placeholder="203.0.113.10"
                      className="bg-card font-mono text-xs md:text-sm"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor={`batch-desired-port-${index}`}>{t("Desired port")}</Label>
                    <Input
                      id={`batch-desired-port-${index}`}
                      {...form.register(`requests.${index}.desiredPort`)}
                      placeholder="10080"
                      className="bg-card font-mono text-xs md:text-sm"
                    />
                  </div>
                  <Controller
                    control={form.control}
                    name={`requests.${index}.countryCodes`}
                    render={({ field }) => (
                      <StringListField
                        id={`batch-country-codes-${index}`}
                        label={t("Country codes")}
                        helper={t("Optional geo scope.")}
                        onChange={field.onChange}
                        placeholder="JP, SG"
                        value={field.value}
                      />
                    )}
                  />
                  <Controller
                    control={form.control}
                    name={`requests.${index}.cities`}
                    render={({ field }) => (
                      <StringListField
                        id={`batch-cities-${index}`}
                        label={t("Cities")}
                        helper={t("Optional city scope.")}
                        onChange={field.onChange}
                        placeholder={t("Enter one city per line")}
                        value={field.value}
                      />
                    )}
                  />
                  <Controller
                    control={form.control}
                    name={`requests.${index}.selectorSpecifiedIps`}
                    render={({ field }) => (
                      <StringListField
                        id={`batch-includes-${index}`}
                        label={t("Selector include list")}
                        helper={t("IPs allowed for this row.")}
                        onChange={field.onChange}
                        placeholder="203.0.113.10"
                        value={field.value}
                      />
                    )}
                  />
                  <Controller
                    control={form.control}
                    name={`requests.${index}.blacklistIps`}
                    render={({ field }) => (
                      <StringListField
                        id={`batch-blacklist-${index}`}
                        label={t("Blacklist")}
                        helper={t("IPs to exclude in this row.")}
                        onChange={field.onChange}
                        placeholder="198.51.100.42"
                        value={field.value}
                      />
                    )}
                  />
                </div>
                <div className="mt-4 grid gap-4 sm:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor={`batch-limit-${index}`}>{t("Selector limit")}</Label>
                    <Input
                      id={`batch-limit-${index}`}
                      {...form.register(`requests.${index}.limit`)}
                      placeholder="1"
                      className="bg-card"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor={`batch-sort-mode-${index}`}>{t("Sort mode")}</Label>
                    <Controller
                      control={form.control}
                      name={`requests.${index}.sortMode`}
                      render={({ field }) => (
                        <Select onValueChange={field.onChange} value={field.value}>
                          <SelectTrigger id={`batch-sort-mode-${index}`} className="w-full bg-card">
                            <SelectValue placeholder={t("Sort mode")} />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="lru">{formatSortMode("lru", t)}</SelectItem>
                            <SelectItem value="mru">{formatSortMode("mru", t)}</SelectItem>
                          </SelectContent>
                        </Select>
                      )}
                    />
                  </div>
                </div>
              </div>
            ))}
          </div>
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-[28px] border border-border/70 bg-[linear-gradient(135deg,rgba(59,130,246,0.08),rgba(20,184,166,0.06))] p-4">
            <Button type="button" variant="outline" onClick={() => fieldArray.append(emptyRow())}>
              <PlusIcon />
              {t("Add request row")}
            </Button>
            <Button disabled={isPending} type="submit" size="lg" className="min-w-40">
              {isPending ? t("Opening batch...") : t("Open batch")}
            </Button>
          </div>
        </form>
        {response ? (
          <ActionResponsePanel
            title={t("Batch opened")}
            description={t("Opened {count} sessions in one transaction.", {
              count: response.sessions.length,
            })}
            bullets={response.sessions.map(
              (session) => `${session.session_id} -> ${session.listen}`,
            )}
          />
        ) : null}
        {error ? (
          <ActionResponsePanel title={t("Batch failed")} description={error} tone="error" />
        ) : null}
      </CardContent>
    </Card>
  );
}
