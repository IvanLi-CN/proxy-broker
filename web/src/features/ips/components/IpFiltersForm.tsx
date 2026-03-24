import { zodResolver } from "@hookform/resolvers/zod";
import { FilterIcon } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

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
import { buildExtractRequest, formatSortMode } from "@/lib/format";
import type { ExtractIpRequest, SortMode } from "@/lib/types";

const schema = z.object({
  countryCodes: z.string(),
  cities: z.string(),
  specifiedIps: z.string(),
  blacklistIps: z.string(),
  limit: z.string(),
  sortMode: z.enum(["mru", "lru"] satisfies SortMode[]),
});

type FormValues = z.infer<typeof schema>;

const defaultValues: FormValues = {
  countryCodes: "JP, US",
  cities: "",
  specifiedIps: "",
  blacklistIps: "",
  limit: "20",
  sortMode: "lru",
};

interface IpFiltersFormProps {
  isPending: boolean;
  onSubmit: (payload: ExtractIpRequest) => void | Promise<void>;
}

export function IpFiltersForm({ isPending, onSubmit }: IpFiltersFormProps) {
  const { t } = useI18n();
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues,
  });

  return (
    <Card className="overflow-hidden border-border/70 bg-card/96 shadow-[0_24px_70px_-44px_rgba(15,23,42,0.55)]">
      <CardHeader className="gap-4 border-b border-border/70 bg-muted/15 pb-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-2">
            <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
              {t("Filter builder")}
            </div>
            <CardTitle className="flex items-center gap-2 text-xl tracking-tight">
              <FilterIcon className="size-4 text-primary" />
              {t("Shape the candidate slice")}
            </CardTitle>
            <CardDescription className="text-sm leading-6 text-muted-foreground md:text-[15px]">
              {t(
                "Start broad with country and city hints, then tighten the set with allow-lists or blacklist fences once probe feedback starts telling a story.",
              )}
            </CardDescription>
          </div>
          <Badge
            variant="outline"
            className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
          >
            {t("Default sort: {sortMode}", { sortMode: formatSortMode("lru", t) })}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-5 pt-5">
        <form
          className="grid gap-5"
          onSubmit={form.handleSubmit(async (values) => {
            await onSubmit(buildExtractRequest(values));
          })}
        >
          <div className="grid gap-4">
            <Controller
              control={form.control}
              name="countryCodes"
              render={({ field }) => (
                <StringListField
                  id="country-codes"
                  label={t("Country codes")}
                  helper={t("Comma or newline separated ISO country codes.")}
                  onChange={field.onChange}
                  placeholder="JP, US, SG"
                  value={field.value}
                />
              )}
            />
            <Controller
              control={form.control}
              name="cities"
              render={({ field }) => (
                <StringListField
                  id="cities"
                  label={t("Cities")}
                  helper={t("Optional city shortlist to bias the result set.")}
                  onChange={field.onChange}
                  placeholder={t("Enter one city per line")}
                  value={field.value}
                />
              )}
            />
            <Controller
              control={form.control}
              name="specifiedIps"
              render={({ field }) => (
                <StringListField
                  id="specified-ips"
                  label={t("Specified IPs")}
                  helper={t("Force-include these IPs before sorting.")}
                  onChange={field.onChange}
                  placeholder="203.0.113.10"
                  value={field.value}
                />
              )}
            />
            <Controller
              control={form.control}
              name="blacklistIps"
              render={({ field }) => (
                <StringListField
                  id="blacklist-ips"
                  label={t("Blacklist IPs")}
                  helper={t("IPs the extractor must exclude.")}
                  onChange={field.onChange}
                  placeholder="198.51.100.42"
                  value={field.value}
                />
              )}
            />
          </div>
          <div className="grid gap-4 rounded-[28px] border border-border/70 bg-background/80 p-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="limit">{t("Limit")}</Label>
              <Input
                id="limit"
                {...form.register("limit")}
                inputMode="numeric"
                placeholder="20"
                className="bg-card"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="sort-mode">{t("Sort mode")}</Label>
              <Controller
                control={form.control}
                name="sortMode"
                render={({ field }) => (
                  <Select onValueChange={field.onChange} value={field.value}>
                    <SelectTrigger id="sort-mode" className="w-full bg-card">
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
            <div className="flex items-end justify-stretch sm:col-span-2 sm:justify-end">
              <Button disabled={isPending} type="submit" size="lg" className="min-w-40">
                {isPending ? t("Extracting...") : t("Extract IPs")}
              </Button>
            </div>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
