import { zodResolver } from "@hookform/resolvers/zod";
import { CableCarIcon } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { StringListField } from "@/components/StringListField";
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
import { buildOpenSessionRequest } from "@/lib/format";
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
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues,
  });

  return (
    <Card className="border-border/70 bg-card/90">
      <CardHeader className="border-b border-border/70">
        <CardTitle className="flex items-center gap-2 text-lg">
          <CableCarIcon className="size-4 text-primary" />
          Open single session
        </CardTitle>
        <CardDescription>
          Pick a single IP or let the selector choose the next best candidate for this profile.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-5 pt-5">
        <form
          className="space-y-4"
          onSubmit={form.handleSubmit(async (values) => {
            await onSubmit(buildOpenSessionRequest(values));
          })}
        >
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="specified-ip">Specified IP</Label>
              <Input
                id="specified-ip"
                {...form.register("specifiedIp")}
                placeholder="203.0.113.10"
                className="font-mono text-xs md:text-sm"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="desired-port">Desired port</Label>
              <Input
                id="desired-port"
                {...form.register("desiredPort")}
                inputMode="numeric"
                placeholder="10080"
                className="font-mono text-xs md:text-sm"
              />
            </div>
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <Controller
              control={form.control}
              name="countryCodes"
              render={({ field }) => (
                <StringListField
                  id="session-country-codes"
                  label="Country codes"
                  helper="Optional selector countries."
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
                  label="Cities"
                  helper="Optional city shortlist."
                  onChange={field.onChange}
                  placeholder="Tokyo"
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
                  label="Selector include list"
                  helper="IPs that remain eligible for selector mode."
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
                  label="Blacklist"
                  helper="IPs the selector must skip."
                  onChange={field.onChange}
                  placeholder="198.51.100.42"
                  value={field.value}
                />
              )}
            />
          </div>
          <div className="grid gap-4 md:grid-cols-[160px_200px_1fr]">
            <div className="space-y-2">
              <Label htmlFor="selector-limit">Selector limit</Label>
              <Input
                id="selector-limit"
                {...form.register("limit")}
                inputMode="numeric"
                placeholder="1"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="session-sort-mode">Sort mode</Label>
              <Controller
                control={form.control}
                name="sortMode"
                render={({ field }) => (
                  <Select onValueChange={field.onChange} value={field.value}>
                    <SelectTrigger id="session-sort-mode" className="w-full">
                      <SelectValue placeholder="Sort mode" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="lru">LRU</SelectItem>
                      <SelectItem value="mru">MRU</SelectItem>
                    </SelectContent>
                  </Select>
                )}
              />
            </div>
            <div className="flex items-end justify-end">
              <Button disabled={isPending} type="submit">
                {isPending ? "Opening..." : "Open session"}
              </Button>
            </div>
          </div>
        </form>
        {response ? (
          <ActionResponsePanel
            title="Session opened"
            description={`Listening on ${response.listen} via ${response.proxy_name} (${response.selected_ip}).`}
            bullets={[`Session ID: ${response.session_id}`, `Port: ${response.port}`]}
          />
        ) : null}
        {error ? (
          <ActionResponsePanel title="Open failed" description={error} tone="error" />
        ) : null}
      </CardContent>
    </Card>
  );
}
