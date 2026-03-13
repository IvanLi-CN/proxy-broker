import { zodResolver } from "@hookform/resolvers/zod";
import { PlusIcon, Rows4Icon, Trash2Icon } from "lucide-react";
import { Controller, useFieldArray, useForm } from "react-hook-form";
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
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      requests: [emptyRow(), { ...emptyRow(), desiredPort: "10081", cities: "Osaka" }],
    },
  });
  const fieldArray = useFieldArray({ control: form.control, name: "requests" });

  return (
    <Card className="border-border/70 bg-card/90">
      <CardHeader className="border-b border-border/70">
        <CardTitle className="flex items-center gap-2 text-lg">
          <Rows4Icon className="size-4 text-primary" />
          Open batch
        </CardTitle>
        <CardDescription>
          Queue multiple open-session requests and let the backend roll back if any row fails.
        </CardDescription>
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
              <div key={field.id} className="rounded-2xl border border-border/70 bg-muted/30 p-4">
                <div className="mb-4 flex items-center justify-between gap-3">
                  <div>
                    <div className="text-sm font-semibold text-foreground">
                      Request #{index + 1}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      Compact selector for one session entry.
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
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor={`batch-specified-ip-${index}`}>Specified IP</Label>
                    <Input
                      id={`batch-specified-ip-${index}`}
                      {...form.register(`requests.${index}.specifiedIp`)}
                      placeholder="203.0.113.10"
                      className="font-mono text-xs md:text-sm"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor={`batch-desired-port-${index}`}>Desired port</Label>
                    <Input
                      id={`batch-desired-port-${index}`}
                      {...form.register(`requests.${index}.desiredPort`)}
                      placeholder="10080"
                      className="font-mono text-xs md:text-sm"
                    />
                  </div>
                  <Controller
                    control={form.control}
                    name={`requests.${index}.countryCodes`}
                    render={({ field }) => (
                      <StringListField
                        id={`batch-country-codes-${index}`}
                        label="Country codes"
                        helper="Optional geo scope."
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
                        label="Cities"
                        helper="Optional city scope."
                        onChange={field.onChange}
                        placeholder="Tokyo"
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
                        label="Selector include list"
                        helper="IPs allowed for this row."
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
                        label="Blacklist"
                        helper="IPs to exclude in this row."
                        onChange={field.onChange}
                        placeholder="198.51.100.42"
                        value={field.value}
                      />
                    )}
                  />
                </div>
                <div className="mt-4 grid gap-4 md:grid-cols-[160px_200px]">
                  <div className="space-y-2">
                    <Label htmlFor={`batch-limit-${index}`}>Selector limit</Label>
                    <Input
                      id={`batch-limit-${index}`}
                      {...form.register(`requests.${index}.limit`)}
                      placeholder="1"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor={`batch-sort-mode-${index}`}>Sort mode</Label>
                    <Controller
                      control={form.control}
                      name={`requests.${index}.sortMode`}
                      render={({ field }) => (
                        <Select onValueChange={field.onChange} value={field.value}>
                          <SelectTrigger id={`batch-sort-mode-${index}`} className="w-full">
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
                </div>
              </div>
            ))}
          </div>
          <div className="flex flex-wrap items-center justify-between gap-3">
            <Button type="button" variant="outline" onClick={() => fieldArray.append(emptyRow())}>
              <PlusIcon />
              Add request row
            </Button>
            <Button disabled={isPending} type="submit">
              {isPending ? "Opening batch..." : "Open batch"}
            </Button>
          </div>
        </form>
        {response ? (
          <ActionResponsePanel
            title="Batch opened"
            description={`Opened ${response.sessions.length} sessions in one transaction.`}
            bullets={response.sessions.map(
              (session) => `${session.session_id} -> ${session.listen}`,
            )}
          />
        ) : null}
        {error ? (
          <ActionResponsePanel title="Batch failed" description={error} tone="error" />
        ) : null}
      </CardContent>
    </Card>
  );
}
