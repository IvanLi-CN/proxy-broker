import { FilterIcon, RotateCcwIcon } from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
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
import type { NodeFilterState } from "@/lib/nodes-view";
import type {
  NodeIpFamilyFilter,
  NodeProbeStatusFilter,
  NodeSessionPresenceFilter,
  NodeSortField,
  SortOrder,
} from "@/lib/types";

interface NodesFiltersBarProps {
  state: NodeFilterState;
  onChange: (patch: Partial<NodeFilterState>) => void;
  onReset: () => void;
}

const probeOptions: Array<{ value: NodeProbeStatusFilter; label: string }> = [
  { value: "any", label: "All probe states" },
  { value: "reachable", label: "Reachable" },
  { value: "unreachable", label: "Unreachable" },
  { value: "unprobed", label: "Unprobed" },
];

const sessionPresenceOptions: Array<{ value: NodeSessionPresenceFilter; label: string }> = [
  { value: "any", label: "All session states" },
  { value: "with_sessions", label: "With sessions" },
  { value: "without_sessions", label: "Without sessions" },
];

const ipFamilyOptions: Array<{ value: NodeIpFamilyFilter; label: string }> = [
  { value: "any", label: "Any family" },
  { value: "ipv4", label: "Has IPv4" },
  { value: "ipv6", label: "Has IPv6" },
  { value: "dual_stack", label: "Dual stack" },
];

const sortOptions: Array<{ value: NodeSortField; label: string }> = [
  { value: "proxy_name", label: "Proxy name" },
  { value: "proxy_type", label: "Proxy type" },
  { value: "preferred_ip", label: "Preferred IP" },
  { value: "region", label: "Region" },
  { value: "latency", label: "Latency" },
  { value: "last_used_at", label: "Last used" },
  { value: "session_count", label: "Session count" },
];

const orderOptions: Array<{ value: SortOrder; label: string }> = [
  { value: "asc", label: "Ascending" },
  { value: "desc", label: "Descending" },
];

export function NodesFiltersBar({ state, onChange, onReset }: NodesFiltersBarProps) {
  const { t } = useI18n();

  return (
    <div className="rounded-[28px] border border-border/70 bg-card/95 p-4 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
          <FilterIcon className="size-4 text-primary" />
          {t("Node filters")}
        </div>
        <Button variant="ghost" size="sm" onClick={onReset}>
          <RotateCcwIcon className="size-4" />
          {t("Reset")}
        </Button>
      </div>

      <div className="grid gap-4 xl:grid-cols-4">
        <Field label={t("Search")}>
          <Input
            value={state.query}
            onChange={(event) => onChange({ query: event.target.value, page: 1 })}
            placeholder={t("proxy name, server, IP, region")}
          />
        </Field>

        <Field label={t("Proxy types")}>
          <Input
            value={state.proxyTypes}
            onChange={(event) => onChange({ proxyTypes: event.target.value, page: 1 })}
            placeholder={t("vmess, trojan")}
          />
        </Field>

        <Field label={t("Country codes")}>
          <Input
            value={state.countryCodes}
            onChange={(event) => onChange({ countryCodes: event.target.value, page: 1 })}
            placeholder={t("JP, US")}
          />
        </Field>

        <Field label={t("Regions")}>
          <Input
            value={state.regions}
            onChange={(event) => onChange({ regions: event.target.value, page: 1 })}
            placeholder={t("Tokyo, California")}
          />
        </Field>

        <Field label={t("Cities")}>
          <Input
            value={state.cities}
            onChange={(event) => onChange({ cities: event.target.value, page: 1 })}
            placeholder={t("Chiyoda, San Jose")}
          />
        </Field>

        <Field label={t("Probe state")}>
          <EnumSelect
            value={state.probeStatus}
            options={probeOptions}
            onValueChange={(value) =>
              onChange({ probeStatus: value as NodeProbeStatusFilter, page: 1 })
            }
          />
        </Field>

        <Field label={t("Session state")}>
          <EnumSelect
            value={state.sessionPresence}
            options={sessionPresenceOptions}
            onValueChange={(value) =>
              onChange({ sessionPresence: value as NodeSessionPresenceFilter, page: 1 })
            }
          />
        </Field>

        <Field label={t("IP family")}>
          <EnumSelect
            value={state.ipFamily}
            options={ipFamilyOptions}
            onValueChange={(value) => onChange({ ipFamily: value as NodeIpFamilyFilter, page: 1 })}
          />
        </Field>

        <Field label={t("Sort by")}>
          <EnumSelect
            value={state.sortBy}
            options={sortOptions}
            onValueChange={(value) => onChange({ sortBy: value as NodeSortField, page: 1 })}
          />
        </Field>

        <Field label={t("Order")}>
          <EnumSelect
            value={state.sortOrder}
            options={orderOptions}
            onValueChange={(value) => onChange({ sortOrder: value as SortOrder, page: 1 })}
          />
        </Field>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="space-y-2">
      <Label>{label}</Label>
      {children}
    </div>
  );
}

function EnumSelect({
  value,
  options,
  onValueChange,
}: {
  value: string;
  options: Array<{ value: string; label: string }>;
  onValueChange: (value: string) => void;
}) {
  return (
    <Select value={value} onValueChange={onValueChange}>
      <SelectTrigger className="bg-background/80">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {options.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
