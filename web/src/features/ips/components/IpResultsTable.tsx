import { LoaderCircleIcon, MapPinnedIcon } from "lucide-react";

import { EmptyPanel } from "@/components/EmptyPanel";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useI18n } from "@/i18n";
import { formatCountryName, formatGeoLabel, formatLatency, formatTimestamp } from "@/lib/format";
import type { ExtractIpItem } from "@/lib/types";
import { cn } from "@/lib/utils";

interface IpResultsTableProps {
  items: ExtractIpItem[];
  isLoading?: boolean;
}

export function IpResultsTable({ items, isLoading }: IpResultsTableProps) {
  const { locale, t } = useI18n();

  if (isLoading && items.length === 0) {
    return (
      <EmptyPanel
        title={t("Extracting candidate IPs")}
        description={t(
          "The backend is applying your filters and assembling the current shortlist.",
        )}
        icon={LoaderCircleIcon}
        hint={t(
          "Results will land here with probe, geo, and last-used metadata once the request returns.",
        )}
      />
    );
  }

  if (items.length === 0) {
    return (
      <EmptyPanel
        title={t("No extracted IPs yet")}
        description={t(
          "Run an extract query to inspect geo hints, probe outcomes, and last-used timestamps.",
        )}
        icon={MapPinnedIcon}
      />
    );
  }

  return (
    <ScrollArea className="rounded-[28px] border border-border/70 bg-card/90 shadow-sm">
      <Table>
        <TableHeader>
          <TableRow className="border-b border-border/70 bg-muted/20">
            <TableHead className="px-4">{t("IP")}</TableHead>
            <TableHead>{t("Geo")}</TableHead>
            <TableHead>{t("Probe")}</TableHead>
            <TableHead>{t("Latency")}</TableHead>
            <TableHead>{t("Last used")}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {items.map((item) => (
            <TableRow key={item.ip} className="[&_td]:py-3">
              <TableCell className="px-4 font-mono text-xs md:text-sm">{item.ip}</TableCell>
              <TableCell>
                <div className="space-y-1">
                  <div className="font-medium">
                    {formatCountryName(locale, item.country_code, item.country_name) ??
                      item.country_code ??
                      t("Unknown")}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {
                      [formatGeoLabel(locale, item.region_name), formatGeoLabel(locale, item.city)]
                        .filter(Boolean)
                        .join(" / ") || t("No city metadata")
                    }
                  </div>
                </div>
              </TableCell>
              <TableCell>
                <Badge
                  variant="outline"
                  className={cn(
                    "rounded-full px-3 py-1 text-[11px] uppercase tracking-[0.14em]",
                    item.probe_ok
                      ? "border-emerald-500/20 bg-emerald-500/[0.08] text-emerald-700 dark:text-emerald-300"
                      : "border-amber-500/25 bg-amber-500/[0.1] text-amber-700 dark:text-amber-300",
                  )}
                >
                  {item.probe_ok ? t("Reachable") : t("Unverified")}
                </Badge>
              </TableCell>
              <TableCell className="font-mono text-xs md:text-sm">
                {formatLatency(locale, t, item.best_latency_ms)}
              </TableCell>
              <TableCell className="text-xs md:text-sm">
                {formatTimestamp(locale, t, item.last_used_at)}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </ScrollArea>
  );
}
