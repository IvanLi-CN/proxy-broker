import { zodResolver } from "@hookform/resolvers/zod";
import { CableCarIcon, ChevronDownIcon, InfoIcon } from "lucide-react";
import { useEffect, useMemo } from "react";
import { Controller, useForm } from "react-hook-form";
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
  formatSortMode,
} from "@/lib/format";
import type {
  OpenSessionRequest,
  OpenSessionResponse,
  SearchSessionOptionsRequest,
  SessionOptionItem,
  SessionSelectionMode,
  SortMode,
} from "@/lib/types";
import { cn } from "@/lib/utils";

type FormValues = {
  selectionMode: SessionSelectionMode;
  desiredPort: string;
  countryCodes: string[];
  cities: string[];
  specifiedIps: string[];
  excludedIps: string[];
  sortMode: SortMode;
};

function createSchema(t: Translator) {
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
    });
}

const defaultValues: FormValues = {
  selectionMode: "any",
  desiredPort: "",
  countryCodes: [],
  cities: [],
  specifiedIps: [],
  excludedIps: [],
  sortMode: "lru",
};

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

type SearchSessionOptionsFn = (
  payload: SearchSessionOptionsRequest,
) => Promise<SessionOptionItem[] | undefined>;

interface OpenSessionFormProps {
  isPending: boolean;
  suggestedPort?: number | null;
  response?: OpenSessionResponse | null;
  error?: string | null;
  defaultAdvancedOpen?: boolean;
  initialValues?: Partial<FormValues>;
  onSubmit: (payload: OpenSessionRequest) => void | Promise<void>;
  searchOptions?: SearchSessionOptionsFn;
}

const emptySearch: SearchSessionOptionsFn = async () => [];

export function OpenSessionForm({
  isPending,
  suggestedPort,
  response,
  error,
  defaultAdvancedOpen = false,
  initialValues,
  onSubmit,
  searchOptions = emptySearch,
}: OpenSessionFormProps) {
  const { t } = useI18n();
  const schema = useMemo(() => createSchema(t), [t]);
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      ...defaultValues,
      ...initialValues,
    },
  });
  const selectionMode = form.watch("selectionMode");
  const countryCodes = form.watch("countryCodes");
  const cities = form.watch("cities");
  const selectionError = form.formState.errors.selectionMode?.message;
  const sortModeOptions: Array<{ value: SortMode; label: string }> = [
    { value: "lru", label: `${formatSortMode("lru", t)} (LRU)` },
    { value: "mru", label: `${formatSortMode("mru", t)} (MRU)` },
  ];

  useEffect(() => {
    const filteredCities = filterCitySelectionsByCountry(cities, countryCodes);
    if (
      filteredCities.length === cities.length &&
      filteredCities.every((value, index) => value === cities[index])
    ) {
      return;
    }
    form.setValue("cities", filteredCities, {
      shouldDirty: true,
      shouldValidate: true,
    });
  }, [cities, countryCodes, form]);

  return (
    <Card className="overflow-hidden border-border/70 bg-card/96 shadow-[0_24px_70px_-44px_rgba(15,23,42,0.55)]">
      <CardHeader className="gap-4 border-b border-border/70 bg-muted/15 pb-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-2">
            <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
              {t("Single open")}
            </div>
            <CardTitle className="flex items-center gap-2 text-xl tracking-tight">
              <CableCarIcon className="size-4 text-primary" />
              {t("Open one listener fast")}
            </CardTitle>
            <CardDescription className="text-sm leading-6 text-muted-foreground md:text-[15px]">
              {t(
                "Pick one simple targeting mode, keep the port optional, and let the backend open the listener from the first surviving candidate.",
              )}
            </CardDescription>
          </div>
          <Badge
            variant="outline"
            className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
          >
            {t("optional port")}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-5 pt-5">
        <form
          className="space-y-4"
          onSubmit={form.handleSubmit(async (values) => {
            await onSubmit(
              buildOpenSessionRequest({
                selectionMode: values.selectionMode,
                desiredPort: values.desiredPort,
                countryCodes: values.countryCodes,
                cities: values.cities,
                specifiedIps: values.specifiedIps,
                excludedIps: values.excludedIps,
                sortMode: values.sortMode,
              }),
            );
          })}
        >
          <div className="space-y-4 rounded-[24px] border border-border/70 bg-background/80 p-4">
            <div className={inlineFieldClass}>
              <div className="md:pt-2">
                <FieldLabel label={t("Targeting mode")} />
              </div>
              <div className="space-y-2">
                <Controller
                  control={form.control}
                  name="selectionMode"
                  render={({ field }) => (
                    <SessionSelectionModeSwitch
                      value={field.value}
                      onChange={field.onChange}
                      size="sm"
                    />
                  )}
                />
                {selectionError ? (
                  <p className="text-xs text-destructive">{selectionError}</p>
                ) : null}
              </div>
            </div>

            {selectionMode !== "any" ? (
              <div className="space-y-3 border-t border-border/70 pt-4">
                {selectionMode === "geo" ? (
                  <>
                    <Controller
                      control={form.control}
                      name="countryCodes"
                      render={({ field }) => (
                        <SearchableMultiSelect
                          id="session-country-codes"
                          label={t("Country")}
                          layout="inline"
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
                      name="cities"
                      render={({ field }) => (
                        <SearchableMultiSelect
                          id="session-cities"
                          label={t("Region / city")}
                          layout="inline"
                          placeholder={t("Search and select cities")}
                          searchPlaceholder={t("Search cities")}
                          emptyText={t("No matching cities")}
                          values={field.value}
                          searchKey={countryCodes.join("|")}
                          onChange={field.onChange}
                          onSearch={async (query) =>
                            (await searchOptions({
                              kind: "city",
                              query,
                              country_codes: countryCodes,
                              limit: 30,
                            })) ?? []
                          }
                        />
                      )}
                    />
                  </>
                ) : null}

                {selectionMode === "ip" ? (
                  <Controller
                    control={form.control}
                    name="specifiedIps"
                    render={({ field }) => (
                      <SearchableMultiSelect
                        id="session-specified-ips"
                        label="IP"
                        layout="inline"
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

            <div
              className={cn(
                pairFieldClass,
                selectionMode !== "any" ? "border-t border-border/70 pt-4" : "",
              )}
            >
              <div className={inlineFieldClass}>
                <div className="md:pt-2">
                  <FieldLabel
                    htmlFor="desired-port"
                    label={t("Port")}
                    hint={t(
                      "Leave blank to auto-allocate; the placeholder shows only the current suggestion and does not reserve it.",
                    )}
                  />
                </div>
                <div>
                  <Input
                    id="desired-port"
                    {...form.register("desiredPort")}
                    inputMode="numeric"
                    placeholder={suggestedPort?.toString() ?? "10080"}
                    className="bg-card font-mono text-xs md:text-sm"
                  />
                </div>
              </div>
              <div className={inlineFieldClass}>
                <div className="md:pt-2">
                  <FieldLabel
                    htmlFor="session-sort-mode"
                    label={t("Selection order")}
                    hint={t("Decides the first match inside the candidate set.")}
                  />
                </div>
                <div>
                  <Controller
                    control={form.control}
                    name="sortMode"
                    render={({ field }) => (
                      <Select onValueChange={field.onChange} value={field.value}>
                        <SelectTrigger id="session-sort-mode" className="w-full bg-card">
                          <SelectValue placeholder={t("Choose a selection order")} />
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
                </div>
              </div>
            </div>
          </div>

          <details
            className="group overflow-hidden rounded-[20px] border border-dashed border-border/70 bg-muted/6"
            open={defaultAdvancedOpen || undefined}
          >
            <summary className="flex list-none items-center justify-between gap-3 px-4 py-2 text-sm font-semibold text-foreground">
              <span className="inline-flex items-center gap-2">
                <span>{t("Advanced")}</span>
                <span className="text-xs font-medium text-muted-foreground">{t("optional")}</span>
              </span>
              <ChevronDownIcon className="size-4 text-muted-foreground transition-transform group-open:rotate-180" />
            </summary>
            <div className="border-t border-border/70 px-0 py-2.5">
              <Controller
                control={form.control}
                name="excludedIps"
                render={({ field }) => (
                  <SearchableMultiSelect
                    id="session-excluded-ips"
                    label={t("Exclude IP")}
                    layout="inline"
                    size="sm"
                    placeholder={t("Select IPs to exclude")}
                    searchPlaceholder={t("Search excluded IPs")}
                    emptyText={t("No matching IPs")}
                    values={field.value}
                    searchKey={`${selectionMode}:${countryCodes.join("|")}:${cities.join("|")}`}
                    onChange={field.onChange}
                    onSearch={async (query) =>
                      (await searchOptions({
                        kind: "ip",
                        query,
                        country_codes: selectionMode === "geo" ? countryCodes : undefined,
                        cities: selectionMode === "geo" ? cities : undefined,
                        limit: 40,
                      })) ?? []
                    }
                  />
                )}
              />
            </div>
          </details>

          <div className="flex items-end justify-stretch sm:justify-end">
            <Button disabled={isPending} type="submit" size="lg" className="min-w-40">
              {isPending ? t("Opening...") : t("Open session")}
            </Button>
          </div>
        </form>
        {response ? (
          <ActionResponsePanel
            title={t("Session opened")}
            description={t("Listening on {listen} via {proxyName} ({selectedIp}).", {
              listen: response.listen,
              proxyName: response.proxy_name,
              selectedIp: response.selected_ip,
            })}
            bullets={[
              t("Session ID: {sessionId}", { sessionId: response.session_id }),
              t("Port: {port}", { port: response.port }),
            ]}
          />
        ) : null}
        {error ? (
          <ActionResponsePanel title={t("Open failed")} description={error} tone="error" />
        ) : null}
      </CardContent>
    </Card>
  );
}
