import { MapPinnedIcon } from "lucide-react";

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
import { formatLatency, formatTimestamp } from "@/lib/format";
import type { ExtractIpItem } from "@/lib/types";

interface IpResultsTableProps {
  items: ExtractIpItem[];
}

export function IpResultsTable({ items }: IpResultsTableProps) {
  if (items.length === 0) {
    return (
      <EmptyPanel
        title="No extracted IPs yet"
        description="Run an extract query to inspect geo hints, probe outcomes, and last-used timestamps."
        icon={MapPinnedIcon}
      />
    );
  }

  return (
    <ScrollArea className="rounded-2xl border border-border/70 bg-card/90 shadow-sm">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>IP</TableHead>
            <TableHead>Geo</TableHead>
            <TableHead>Probe</TableHead>
            <TableHead>Latency</TableHead>
            <TableHead>Last used</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {items.map((item) => (
            <TableRow key={item.ip}>
              <TableCell className="font-mono text-xs md:text-sm">{item.ip}</TableCell>
              <TableCell>
                <div className="space-y-1">
                  <div className="font-medium">
                    {item.country_name ?? item.country_code ?? "Unknown"}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {[item.region_name, item.city].filter(Boolean).join(" / ") ||
                      "No city metadata"}
                  </div>
                </div>
              </TableCell>
              <TableCell>
                <Badge variant={item.probe_ok ? "secondary" : "outline"}>
                  {item.probe_ok ? "Reachable" : "Unverified"}
                </Badge>
              </TableCell>
              <TableCell>{formatLatency(item.best_latency_ms)}</TableCell>
              <TableCell>{formatTimestamp(item.last_used_at)}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </ScrollArea>
  );
}
