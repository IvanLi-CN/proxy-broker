import { zodResolver } from "@hookform/resolvers/zod";
import { FilterIcon } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

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
import { buildExtractRequest } from "@/lib/format";
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
  cities: "Tokyo",
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
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues,
  });

  return (
    <Card className="border-border/70 bg-card/90">
      <CardHeader className="border-b border-border/70">
        <CardTitle className="flex items-center gap-2 text-lg">
          <FilterIcon className="size-4 text-primary" />
          Extract IPs
        </CardTitle>
        <CardDescription>
          Build an operator slice of the IP pool by geo hints, allow-lists, and LRU/MRU ordering.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-5 pt-5">
        <form
          className="grid gap-4"
          onSubmit={form.handleSubmit(async (values) => {
            await onSubmit(buildExtractRequest(values));
          })}
        >
          <div className="grid gap-4 md:grid-cols-2">
            <Controller
              control={form.control}
              name="countryCodes"
              render={({ field }) => (
                <StringListField
                  id="country-codes"
                  label="Country codes"
                  helper="Comma or newline separated ISO country codes."
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
                  label="Cities"
                  helper="Optional city shortlist to bias the result set."
                  onChange={field.onChange}
                  placeholder="Tokyo\nOsaka"
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
                  label="Specified IPs"
                  helper="Force include these IPs before sorting."
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
                  label="Blacklist IPs"
                  helper="IPs to exclude from the response."
                  onChange={field.onChange}
                  placeholder="198.51.100.42"
                  value={field.value}
                />
              )}
            />
          </div>
          <div className="grid gap-4 md:grid-cols-[160px_200px_1fr]">
            <div className="space-y-2">
              <Label htmlFor="limit">Limit</Label>
              <Input id="limit" {...form.register("limit")} inputMode="numeric" placeholder="20" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="sort-mode">Sort mode</Label>
              <Controller
                control={form.control}
                name="sortMode"
                render={({ field }) => (
                  <Select onValueChange={field.onChange} value={field.value}>
                    <SelectTrigger id="sort-mode" className="w-full">
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
                {isPending ? "Extracting..." : "Extract IPs"}
              </Button>
            </div>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
