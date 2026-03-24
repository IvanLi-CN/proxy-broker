import { zodResolver } from "@hookform/resolvers/zod";
import { CableCarIcon } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
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
import type { OpenSessionRequest, OpenSessionResponse, SortMode } from "@/lib/types";

const schema = z.object({
  specifiedIp: z.string(),
  desiredPort: z.string(),
  countryCodes: z.string(),
  cities: z.string(),
  selectorSpecifiedIps: z.string(),
  blacklistIps: z.string(),
  limit: z.string(),
  sortMode: z.enum(["mru", "lru"] satisfies SortMode[]),
});

type FormValues = z.infer<typeof schema>;

const defaultValues: FormValues = {
  specifiedIp: "",
  desiredPort: "10080",
  countryCodes: "JP",
  cities: "",
  selectorSpecifiedIps: "",
  blacklistIps: "",
  limit: "1",
  sortMode: "lru",
};

interface OpenSessionFormProps {
  isPending: boolean;
  response?: OpenSessionResponse | null;
  error?: string | null;
  onSubmit: (payload: OpenSessionRequest) => void | Promise<void>;
}

export function OpenSessionForm({ isPending, response, error, onSubmit }: OpenSessionFormProps) {
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
              {t("Single open")}
            </div>
            <CardTitle className="flex items-center gap-2 text-xl tracking-tight">
              <CableCarIcon className="size-4 text-primary" />
              {t("Open one listener fast")}
            </CardTitle>
            <CardDescription className="text-sm leading-6 text-muted-foreground md:text-[15px]">
              {t(
                "Pin a specific IP when you know exactly what you want, or let the selector pick the next best edge for the active profile.",
              )}
            </CardDescription>
          </div>
          <Badge
            variant="outline"
            className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
          >
            {t("selector limit 1")}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-5 pt-5">
        <form
          className="space-y-4"
          onSubmit={form.handleSubmit(async (values) => {
            await onSubmit(buildOpenSessionRequest(values));
          })}
        >
          <div className="grid gap-4 rounded-[28px] border border-border/70 bg-background/80 p-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="specified-ip">{t("Specified IP")}</Label>
              <Input
                id="specified-ip"
                {...form.register("specifiedIp")}
                placeholder="203.0.113.10"
                className="bg-card font-mono text-xs md:text-sm"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="desired-port">{t("Desired port")}</Label>
              <Input
                id="desired-port"
                {...form.register("desiredPort")}
                inputMode="numeric"
                placeholder="10080"
                className="bg-card font-mono text-xs md:text-sm"
              />
            </div>
          </div>
          <div className="grid gap-4">
            <Controller
              control={form.control}
              name="countryCodes"
              render={({ field }) => (
                <StringListField
                  id="session-country-codes"
                  label={t("Country codes")}
                  helper={t("Optional selector countries.")}
                  onChange={field.onChange}
                  placeholder="JP, SG"
                  value={field.value}
                />
              )}
            />
            <Controller
              control={form.control}
              name="cities"
              render={({ field }) => (
                <StringListField
                  id="session-cities"
                  label={t("Cities")}
                  helper={t("Optional city shortlist.")}
                  onChange={field.onChange}
                  placeholder={t("Enter one city per line")}
                  value={field.value}
                />
              )}
            />
            <Controller
              control={form.control}
              name="selectorSpecifiedIps"
              render={({ field }) => (
                <StringListField
                  id="selector-specified-ips"
                  label={t("Selector include list")}
                  helper={t("IPs that remain eligible for selector mode.")}
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
                  id="selector-blacklist-ips"
                  label={t("Blacklist")}
                  helper={t("IPs the selector must skip.")}
                  onChange={field.onChange}
                  placeholder="198.51.100.42"
                  value={field.value}
                />
              )}
            />
          </div>
          <div className="grid gap-4 rounded-[28px] border border-border/70 bg-background/80 p-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="selector-limit">{t("Selector limit")}</Label>
              <Input
                id="selector-limit"
                {...form.register("limit")}
                inputMode="numeric"
                placeholder="1"
                className="bg-card"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="session-sort-mode">{t("Sort mode")}</Label>
              <Controller
                control={form.control}
                name="sortMode"
                render={({ field }) => (
                  <Select onValueChange={field.onChange} value={field.value}>
                    <SelectTrigger id="session-sort-mode" className="w-full bg-card">
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
                {isPending ? t("Opening...") : t("Open session")}
              </Button>
            </div>
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
