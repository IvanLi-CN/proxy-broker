import { zodResolver } from "@hookform/resolvers/zod";
import { ChevronDownIcon, InfoIcon, PlusIcon, Rows4Icon, Trash2Icon } from "lucide-react";
import { useEffect, useMemo } from "react";
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
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { SessionSelectionModeSwitch } from "@/features/sessions/components/SessionSelectionModeSwitch";
import { type Translator, useI18n } from "@/i18n";
import {
  buildOpenSessionRequest,
  filterCitySelectionsByCountry,
  findOverlappingValues,
  formatSortMode,
} from "@/lib/format";
import type {
  OpenBatchRequest,
  OpenBatchResponse,
  SearchSessionOptionsRequest,
  SessionOptionItem,
  SessionSelectionMode,
  SortMode,
} from "@/lib/types";

type BatchRequestRow = {
  selectionMode: SessionSelectionMode;
  desiredPort: string;
  countryCodes: string[];
  cities: string[];
  specifiedIps: string[];
  excludedIps: string[];
  sortMode: SortMode;
};

type FormValues = {
  requests: BatchRequestRow[];
};

function createRowSchema(t: Translator) {
  return z
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
          message: t("Choose at least one country or city."),
        });
      }
      if (value.selectionMode === "ip" && value.specifiedIps.length === 0) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ["selectionMode"],
          message: t("Choose at least one IP."),
        });
      }
      if (value.selectionMode === "ip") {
        const overlaps = findOverlappingValues(value.specifiedIps, value.excludedIps);
        if (overlaps.length > 0) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ["selectionMode"],
            message: t("The same IP cannot appear in both include and exclude lists."),
          });
        }
      }
    });
}

function createSchema(t: Translator) {
  return z.object({
    requests: z.array(createRowSchema(t)).min(1),
  });
}

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

const inlineFieldClass = "grid gap-2 md:grid-cols-[88px_minmax(0,1fr)] md:items-start md:gap-3";
const pairFieldClass = "grid gap-3 lg:grid-cols-2";

function FieldLabel({ htmlFor, label, hint }: { htmlFor?: string; label: string; hint?: string }) {
  const { t } = useI18n();
  return (
    <div className="inline-flex items-center gap-1.5">
      <Label htmlFor={htmlFor}>{label}</Label>
      {hint ? (
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              type="button"
              className="inline-flex size-4 items-center justify-center rounded-full text-muted-foreground transition-colors hover:text-foreground"
              aria-label={t("More about {label}", { label })}
            >
              <InfoIcon className="size-3.5" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="top" align="start" sideOffset={6}>
            {hint}
          </TooltipContent>
        </Tooltip>
      ) : null}
    </div>
  );
}

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
  const { t } = useI18n();
  const schema = useMemo(() => createSchema(t), [t]);
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
  const sortModeOptions: Array<{ value: SortMode; label: string }> = [
    { value: "lru", label: `${formatSortMode("lru", t)} (LRU)` },
    { value: "mru", label: `${formatSortMode("mru", t)} (MRU)` },
  ];

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
              {t(
                "Stage multiple open-session requests with the same simplified targeting model, then let the backend roll the whole set back if any row fails.",
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
                        {t("Request #{index}", { index: index + 1 })}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        {t("One row, one listener; all rows still succeed or fail together.")}
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

                  <div className="space-y-3">
                    <div className={inlineFieldClass}>
                      <div className="md:pt-1.5">
                        <FieldLabel label={t("Targeting mode")} />
                      </div>
                      <div className="space-y-2">
                        <Controller
                          control={form.control}
                          name={`requests.${index}.selectionMode`}
                          render={({ field }) => (
                            <SessionSelectionModeSwitch
                              value={field.value}
                              onChange={field.onChange}
                              size="sm"
                            />
                          )}
                        />
                        {rowSelectionError ? (
                          <p className="text-xs text-destructive">{rowSelectionError}</p>
                        ) : null}
                      </div>
                    </div>

                    {row.selectionMode !== "any" ? (
                      <div className="space-y-3 border-t border-border/70 pt-3">
                        {row.selectionMode === "geo" ? (
                          <>
                            <Controller
                              control={form.control}
                              name={`requests.${index}.countryCodes`}
                              render={({ field }) => (
                                <SearchableMultiSelect
                                  id={`batch-country-codes-${index}`}
                                  label={t("Country")}
                                  layout="inline"
                                  size="sm"
                                  placeholder={t("Search and select countries")}
                                  searchPlaceholder={t("Search countries or codes")}
                                  emptyText={t("No matching countries")}
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
                                  label={t("Region / city")}
                                  layout="inline"
                                  size="sm"
                                  placeholder={t("Search and select cities")}
                                  searchPlaceholder={t("Search cities")}
                                  emptyText={t("No matching cities")}
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
                                layout="inline"
                                size="sm"
                                placeholder={t("Search and select IPs")}
                                searchPlaceholder={t("Search IPs")}
                                emptyText={t("No matching IPs")}
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
                    ) : null}

                    <div className={`${pairFieldClass} border-t border-border/70 pt-3`}>
                      <div className={inlineFieldClass}>
                        <div className="md:pt-1.5">
                          <FieldLabel
                            htmlFor={`batch-desired-port-${index}`}
                            label={t("Port")}
                            hint={t("Leave blank to auto-allocate.")}
                          />
                        </div>
                        <div>
                          <Input
                            id={`batch-desired-port-${index}`}
                            {...form.register(`requests.${index}.desiredPort`)}
                            size="sm"
                            placeholder={suggestedPort?.toString() ?? "10080"}
                            className="bg-card font-mono text-xs"
                          />
                        </div>
                      </div>
                      <div className={inlineFieldClass}>
                        <div className="md:pt-1.5">
                          <FieldLabel
                            htmlFor={`batch-sort-mode-${index}`}
                            label={t("Selection order")}
                            hint={t("Only decides the first match for this row.")}
                          />
                        </div>
                        <div>
                          <Controller
                            control={form.control}
                            name={`requests.${index}.sortMode`}
                            render={({ field }) => (
                              <Select onValueChange={field.onChange} value={field.value}>
                                <SelectTrigger
                                  id={`batch-sort-mode-${index}`}
                                  size="sm"
                                  className="w-full bg-card"
                                >
                                  <SelectValue placeholder={t("Choose a selection order")} />
                                </SelectTrigger>
                                <SelectContent size="sm">
                                  {sortModeOptions.map((option) => (
                                    <SelectItem key={option.value} value={option.value} size="sm">
                                      {option.label}
                                    </SelectItem>
                                  ))}
                                </SelectContent>
                              </Select>
                            )}
                          />
                        </div>
                      </div>
                    </div>
                  </div>

                  <details
                    className="group mt-3 overflow-hidden rounded-[20px] border border-dashed border-border/70 bg-muted/6"
                    open={defaultAdvancedOpen || undefined}
                  >
                    <summary className="flex list-none items-center justify-between gap-3 px-4 py-2 text-sm font-semibold text-foreground">
                      <span className="inline-flex items-center gap-2">
                        <span>{t("Advanced")}</span>
                        <span className="text-xs font-medium text-muted-foreground">
                          {t("optional")}
                        </span>
                      </span>
                      <ChevronDownIcon className="size-4 text-muted-foreground transition-transform group-open:rotate-180" />
                    </summary>
                    <div className="border-t border-border/70 px-0 py-2.5">
                      <Controller
                        control={form.control}
                        name={`requests.${index}.excludedIps`}
                        render={({ field }) => (
                          <SearchableMultiSelect
                            id={`batch-excluded-ips-${index}`}
                            label={t("Exclude IP")}
                            layout="inline"
                            size="sm"
                            placeholder={t("Select IPs to exclude")}
                            searchPlaceholder={t("Search excluded IPs")}
                            emptyText={t("No matching IPs")}
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
