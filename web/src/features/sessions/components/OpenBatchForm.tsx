import { zodResolver } from "@hookform/resolvers/zod";
import { ChevronDownIcon, PlusIcon, Rows4Icon, Trash2Icon } from "lucide-react";
import { useEffect } from "react";
import { Controller, useFieldArray, useForm } from "react-hook-form";
import { z } from "zod";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { SearchableMultiSelect } from "@/components/SearchableMultiSelect";
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
import { buildOpenSessionRequest, filterCitySelectionsByCountry } from "@/lib/format";
import type {
  OpenBatchRequest,
  OpenBatchResponse,
  SearchSessionOptionsRequest,
  SessionOptionItem,
  SessionSelectionMode,
  SortMode,
} from "@/lib/types";
import { cn } from "@/lib/utils";

const selectionModeOptions: Array<{
  value: SessionSelectionMode;
  title: string;
  description: string;
}> = [
  {
    value: "any",
    title: "不限",
    description: "全部候选里按顺序挑第一条。",
  },
  {
    value: "geo",
    title: "国家/地区",
    description: "收窄到国家或城市。",
  },
  {
    value: "ip",
    title: "IP",
    description: "直接圈定一个或多个 IP。",
  },
];

const sortModeOptions: Array<{ value: SortMode; label: string }> = [
  { value: "lru", label: "最久未使用优先 (LRU)" },
  { value: "mru", label: "最近使用优先 (MRU)" },
];

const rowSchema = z
  .object({
    selectionMode: z.enum(["any", "geo", "ip"] satisfies SessionSelectionMode[]),
    desiredPort: z.string(),
    countryCodes: z.array(z.string()),
    cities: z.array(z.string()),
    specifiedIps: z.array(z.string()),
    excludedIps: z.array(z.string()),
    sortMode: z.enum(["mru", "lru"] satisfies SortMode[]),
  })
  .superRefine((value, ctx) => {
    if (
      value.selectionMode === "geo" &&
      value.countryCodes.length === 0 &&
      value.cities.length === 0
    ) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["selectionMode"],
        message: "至少选择 1 个国家或城市。",
      });
    }
    if (value.selectionMode === "ip" && value.specifiedIps.length === 0) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["selectionMode"],
        message: "至少选择 1 个 IP。",
      });
    }
  });

const schema = z.object({
  requests: z.array(rowSchema).min(1),
});

type BatchRequestRow = z.infer<typeof rowSchema>;
type FormValues = z.infer<typeof schema>;

type SearchSessionOptionsFn = (
  payload: SearchSessionOptionsRequest,
) => Promise<SessionOptionItem[] | undefined>;

const emptyRow = (): BatchRequestRow => ({
  selectionMode: "any",
  desiredPort: "",
  countryCodes: [],
  cities: [],
  specifiedIps: [],
  excludedIps: [],
  sortMode: "lru",
});

interface OpenBatchFormProps {
  isPending: boolean;
  suggestedPort?: number | null;
  response?: OpenBatchResponse | null;
  error?: string | null;
  defaultAdvancedOpen?: boolean;
  initialRequests?: BatchRequestRow[];
  onSubmit: (payload: OpenBatchRequest) => void | Promise<void>;
  searchOptions?: SearchSessionOptionsFn;
}

const emptySearch: SearchSessionOptionsFn = async () => [];

export function OpenBatchForm({
  isPending,
  suggestedPort,
  response,
  error,
  defaultAdvancedOpen = false,
  initialRequests,
  onSubmit,
  searchOptions = emptySearch,
}: OpenBatchFormProps) {
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      requests:
        initialRequests && initialRequests.length > 0
          ? initialRequests
          : [emptyRow(), { ...emptyRow(), selectionMode: "geo", cities: ["Osaka"] }],
    },
  });
  const fieldArray = useFieldArray({ control: form.control, name: "requests" });
  const watchedRequests = form.watch("requests");

  useEffect(() => {
    watchedRequests.forEach((row, index) => {
      const filteredCities = filterCitySelectionsByCountry(row.cities, row.countryCodes);
      if (
        filteredCities.length === row.cities.length &&
        filteredCities.every((value, cityIndex) => value === row.cities[cityIndex])
      ) {
        return;
      }
      form.setValue(`requests.${index}.cities`, filteredCities, {
        shouldDirty: true,
        shouldValidate: true,
      });
    });
  }, [form, watchedRequests]);

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
              Stage multiple open-session requests with the same simplified targeting model, then
              let the backend roll the whole set back if any row fails.
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
              requests: values.requests.map((row: BatchRequestRow) =>
                buildOpenSessionRequest({
                  selectionMode: row.selectionMode,
                  desiredPort: row.desiredPort,
                  countryCodes: row.countryCodes,
                  cities: row.cities,
                  specifiedIps: row.specifiedIps,
                  excludedIps: row.excludedIps,
                  sortMode: row.sortMode,
                }),
              ),
            });
          })}
        >
          <div className="space-y-4">
            {fieldArray.fields.map((field, index) => {
              const row = watchedRequests[index] ?? emptyRow();
              const rowSelectionError =
                form.formState.errors.requests?.[index]?.selectionMode?.message;
              return (
                <div
                  key={field.id}
                  className="rounded-[28px] border border-border/70 bg-background/80 p-4"
                >
                  <div className="mb-4 flex items-center justify-between gap-3">
                    <div>
                      <div className="text-sm font-semibold text-foreground">
                        Request #{index + 1}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        One row, one listener; all rows still succeed or fail together.
                      </div>
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-sm"
                      onClick={() => fieldArray.remove(index)}
                      disabled={fieldArray.fields.length === 1}
                      aria-label={`Remove request ${index + 1}`}
                    >
                      <Trash2Icon />
                    </Button>
                  </div>

                  <div className="space-y-3 rounded-[24px] border border-border/70 bg-card/50 p-4">
                    <div className="space-y-1">
                      <Label>选择范围</Label>
                      <p className="text-xs text-muted-foreground">
                        先选这一行的候选来源，再决定是否指定端口。
                      </p>
                    </div>
                    <Controller
                      control={form.control}
                      name={`requests.${index}.selectionMode`}
                      render={({ field }) => (
                        <div className="grid gap-3 md:grid-cols-3">
                          {selectionModeOptions.map((option) => {
                            const active = field.value === option.value;
                            return (
                              <button
                                key={option.value}
                                type="button"
                                className={cn(
                                  "rounded-[22px] border px-4 py-3 text-left transition-colors",
                                  active
                                    ? "border-primary bg-primary/8 shadow-sm"
                                    : "border-border/70 bg-background hover:border-primary/35 hover:bg-muted/40",
                                )}
                                onClick={() => field.onChange(option.value)}
                              >
                                <div className="text-sm font-semibold text-foreground">
                                  {option.title}
                                </div>
                                <div className="mt-1 text-xs leading-5 text-muted-foreground">
                                  {option.description}
                                </div>
                              </button>
                            );
                          })}
                        </div>
                      )}
                    />
                    {rowSelectionError ? (
                      <p className="text-xs text-destructive">{rowSelectionError}</p>
                    ) : null}
                  </div>

                  <div className="mt-4 grid gap-4">
                    {row.selectionMode === "any" ? (
                      <div className="rounded-[22px] border border-dashed border-border/70 bg-card/60 px-4 py-3 text-sm leading-6 text-muted-foreground">
                        不限模式会直接从这行当前 profile 的全部候选里，按照提取顺序选第 1 个。
                      </div>
                    ) : null}

                    {row.selectionMode === "geo" ? (
                      <>
                        <Controller
                          control={form.control}
                          name={`requests.${index}.countryCodes`}
                          render={({ field }) => (
                            <SearchableMultiSelect
                              id={`batch-country-codes-${index}`}
                              label="国家"
                              helper="可多选；城市搜索会按这里的结果收窄。"
                              placeholder="选择国家"
                              searchPlaceholder="搜索国家或代码"
                              emptyText="No matching countries"
                              values={field.value}
                              onChange={field.onChange}
                              onSearch={async (query) =>
                                (await searchOptions({
                                  kind: "country",
                                  query,
                                  limit: 20,
                                })) ?? []
                              }
                            />
                          )}
                        />
                        <Controller
                          control={form.control}
                          name={`requests.${index}.cities`}
                          render={({ field }) => (
                            <SearchableMultiSelect
                              id={`batch-cities-${index}`}
                              label="地区 / 城市"
                              helper="可选；不填就只按国家过滤。"
                              placeholder="选择城市"
                              searchPlaceholder="搜索城市"
                              emptyText="No matching cities"
                              values={field.value}
                              searchKey={row.countryCodes.join("|")}
                              onChange={field.onChange}
                              onSearch={async (query) =>
                                (await searchOptions({
                                  kind: "city",
                                  query,
                                  country_codes: row.countryCodes,
                                  limit: 30,
                                })) ?? []
                              }
                            />
                          )}
                        />
                      </>
                    ) : null}

                    {row.selectionMode === "ip" ? (
                      <Controller
                        control={form.control}
                        name={`requests.${index}.specifiedIps`}
                        render={({ field }) => (
                          <SearchableMultiSelect
                            id={`batch-specified-ips-${index}`}
                            label="IP"
                            helper="可多选；最终会按提取顺序从这些 IP 里挑第 1 个。"
                            placeholder="选择 IP"
                            searchPlaceholder="搜索 IP"
                            emptyText="No matching IPs"
                            values={field.value}
                            onChange={field.onChange}
                            onSearch={async (query) =>
                              (await searchOptions({
                                kind: "ip",
                                query,
                                limit: 40,
                              })) ?? []
                            }
                          />
                        )}
                      />
                    ) : null}
                  </div>

                  <div className="mt-4 grid gap-4 rounded-[24px] border border-border/70 bg-card/50 p-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <Label htmlFor={`batch-desired-port-${index}`}>Desired port</Label>
                      <Input
                        id={`batch-desired-port-${index}`}
                        {...form.register(`requests.${index}.desiredPort`)}
                        placeholder={suggestedPort?.toString() ?? "10080"}
                        className="bg-card font-mono text-xs md:text-sm"
                      />
                      <p className="text-xs text-muted-foreground">
                        留空时这一行也会自动分配端口。
                      </p>
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor={`batch-sort-mode-${index}`}>提取顺序</Label>
                      <Controller
                        control={form.control}
                        name={`requests.${index}.sortMode`}
                        render={({ field }) => (
                          <Select onValueChange={field.onChange} value={field.value}>
                            <SelectTrigger id={`batch-sort-mode-${index}`} className="w-full bg-card">
                              <SelectValue placeholder="选择提取顺序" />
                            </SelectTrigger>
                            <SelectContent>
                              {sortModeOptions.map((option) => (
                                <SelectItem key={option.value} value={option.value}>
                                  {option.label}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        )}
                      />
                      <p className="text-xs text-muted-foreground">
                        只决定这行候选集合里的第 1 个命中项。
                      </p>
                    </div>
                  </div>

                  <details
                    className="mt-4 rounded-[24px] border border-border/70 bg-card/40"
                    open={defaultAdvancedOpen || undefined}
                  >
                    <summary className="flex list-none items-center justify-between gap-3 px-4 py-3 text-sm font-semibold text-foreground">
                      Advanced
                      <ChevronDownIcon className="size-4 text-muted-foreground" />
                    </summary>
                    <div className="border-t border-border/70 px-4 py-4">
                      <Controller
                        control={form.control}
                        name={`requests.${index}.excludedIps`}
                        render={({ field }) => (
                          <SearchableMultiSelect
                            id={`batch-excluded-ips-${index}`}
                            label="排除 IP"
                            helper="可选；这些 IP 会被这一行明确跳过。"
                            placeholder="选择要排除的 IP"
                            searchPlaceholder="搜索要排除的 IP"
                            emptyText="No matching IPs"
                            values={field.value}
                            searchKey={`${row.selectionMode}:${row.countryCodes.join("|")}:${row.cities.join("|")}`}
                            onChange={field.onChange}
                            onSearch={async (query) =>
                              (await searchOptions({
                                kind: "ip",
                                query,
                                country_codes:
                                  row.selectionMode === "geo" ? row.countryCodes : undefined,
                                cities: row.selectionMode === "geo" ? row.cities : undefined,
                                limit: 40,
                              })) ?? []
                            }
                          />
                        )}
                      />
                    </div>
                  </details>
                </div>
              );
            })}
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
