import { zodResolver } from "@hookform/resolvers/zod";
import { CableCarIcon, ChevronDownIcon } from "lucide-react";
import { useEffect } from "react";
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
import { buildOpenSessionRequest, filterCitySelectionsByCountry } from "@/lib/format";
import type {
  OpenSessionRequest,
  OpenSessionResponse,
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
    description: "从当前 profile 的全部候选里直接挑一个。",
  },
  {
    value: "geo",
    title: "国家/地区",
    description: "先收窄到国家或城市，再按顺序挑第一条。",
  },
  {
    value: "ip",
    title: "IP",
    description: "手动圈定一个或多个 IP，由顺序字段决定命中项。",
  },
];

const sortModeOptions: Array<{ value: SortMode; label: string }> = [
  { value: "lru", label: "最久未使用优先 (LRU)" },
  { value: "mru", label: "最近使用优先 (MRU)" },
];

const schema = z
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

type FormValues = z.infer<typeof schema>;

const defaultValues: FormValues = {
  selectionMode: "any",
  desiredPort: "",
  countryCodes: [],
  cities: [],
  specifiedIps: [],
  excludedIps: [],
  sortMode: "lru",
};

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
              Pick one simple targeting mode, keep the port optional, and let the backend open the
              listener from the first surviving candidate.
            </CardDescription>
          </div>
          <Badge
            variant="outline"
            className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
          >
            optional port
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
          <div className="space-y-3 rounded-[28px] border border-border/70 bg-background/80 p-4">
            <div className="space-y-1">
              <Label>选择范围</Label>
              <p className="text-xs text-muted-foreground">
                三选一：不限、国家/地区、IP。先决定候选集合，再让顺序字段挑第一条。
              </p>
            </div>
            <Controller
              control={form.control}
              name="selectionMode"
              render={({ field }) => (
                <div className="grid gap-3 md:grid-cols-3">
                  {selectionModeOptions.map((option) => {
                    const active = field.value === option.value;
                    return (
                      <button
                        key={option.value}
                        type="button"
                        className={cn(
                          "rounded-[24px] border px-4 py-3 text-left transition-colors",
                          active
                            ? "border-primary bg-primary/8 shadow-sm"
                            : "border-border/70 bg-card hover:border-primary/35 hover:bg-muted/40",
                        )}
                        onClick={() => field.onChange(option.value)}
                      >
                        <div className="text-sm font-semibold text-foreground">{option.title}</div>
                        <div className="mt-1 text-xs leading-5 text-muted-foreground">
                          {option.description}
                        </div>
                      </button>
                    );
                  })}
                </div>
              )}
            />
            {selectionError ? <p className="text-xs text-destructive">{selectionError}</p> : null}
          </div>

          <div className="grid gap-4 rounded-[28px] border border-border/70 bg-background/80 p-4">
            {selectionMode === "any" ? (
              <div className="rounded-[22px] border border-dashed border-border/70 bg-card/60 px-4 py-3 text-sm leading-6 text-muted-foreground">
                不限模式会从当前 profile 的全部候选 IP 中，按照你设置的提取顺序直接选第
                1 个。
              </div>
            ) : null}

            {selectionMode === "geo" ? (
              <>
                <Controller
                  control={form.control}
                  name="countryCodes"
                  render={({ field }) => (
                    <SearchableMultiSelect
                      id="session-country-codes"
                      label="国家"
                      helper="支持搜索与多选；城市候选会跟随这里的选择收窄。"
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
                  name="cities"
                  render={({ field }) => (
                    <SearchableMultiSelect
                      id="session-cities"
                      label="地区 / 城市"
                      helper="可选；如果不填，就只按国家过滤。"
                      placeholder="选择城市"
                      searchPlaceholder="搜索城市"
                      emptyText="No matching cities"
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
                    helper="可多选；最终会按提取顺序从这里面挑第 1 个可用项。"
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

          <div className="grid gap-4 rounded-[28px] border border-border/70 bg-background/80 p-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="desired-port">{t("Desired port")}</Label>
              <Input
                id="desired-port"
                {...form.register("desiredPort")}
                inputMode="numeric"
                placeholder={suggestedPort?.toString() ?? "10080"}
                className="bg-card font-mono text-xs md:text-sm"
              />
              <p className="text-xs text-muted-foreground">
                留空也能创建；placeholder 只显示当前建议端口，不会预留它。
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="session-sort-mode">提取顺序</Label>
              <Controller
                control={form.control}
                name="sortMode"
                render={({ field }) => (
                  <Select onValueChange={field.onChange} value={field.value}>
                    <SelectTrigger id="session-sort-mode" className="w-full bg-card">
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
                用它决定候选集合里的第 1 个命中项，不再显示 selector limit。
              </p>
            </div>
          </div>

          <details
            className="rounded-[28px] border border-border/70 bg-background/80"
            open={defaultAdvancedOpen || undefined}
          >
            <summary className="flex list-none items-center justify-between gap-3 px-4 py-3 text-sm font-semibold text-foreground">
              Advanced
              <ChevronDownIcon className="size-4 text-muted-foreground transition-transform group-open:rotate-180" />
            </summary>
            <div className="border-t border-border/70 px-4 py-4">
              <Controller
                control={form.control}
                name="excludedIps"
                render={({ field }) => (
                  <SearchableMultiSelect
                    id="session-excluded-ips"
                    label="排除 IP"
                    helper="可选；这些 IP 会被明确跳过。若与 IP 模式已选项冲突，后端会返回错误。"
                    placeholder="选择要排除的 IP"
                    searchPlaceholder="搜索要排除的 IP"
                    emptyText="No matching IPs"
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
              {isPending ? "Opening..." : "Open session"}
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
