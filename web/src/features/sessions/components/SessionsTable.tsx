import { LoaderCircleIcon, PlugZapIcon } from "lucide-react";

import { EmptyPanel } from "@/components/EmptyPanel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useI18n } from "@/i18n";
import { formatTimestamp } from "@/lib/format";
import type { SessionRecord } from "@/lib/types";
import { cn } from "@/lib/utils";

interface SessionsTableProps {
  sessions: SessionRecord[];
  isLoading?: boolean;
  closingSessionId?: string | null;
  onCloseSession: (sessionId: string) => void;
}

export function SessionsTable({
  sessions,
  isLoading,
  closingSessionId,
  onCloseSession,
}: SessionsTableProps) {
  const { locale, t } = useI18n();

  if (isLoading && sessions.length === 0) {
    return (
      <EmptyPanel
        title={t("Loading sessions")}
        description={t("Polling the backend for active listeners on this profile.")}
        icon={LoaderCircleIcon}
        hint={t(
          "The live listener inventory will appear here as soon as the first response lands.",
        )}
      />
    );
  }

  if (sessions.length === 0) {
    return (
      <EmptyPanel
        title={t("No active sessions")}
        description={t("Open a single session or a batch to populate this listener deck.")}
        icon={PlugZapIcon}
      />
    );
  }

  return (
    <ScrollArea className="rounded-[28px] border border-border/70 bg-card/90 shadow-sm">
      <Table className="min-w-[860px]">
        <TableHeader>
          <TableRow className="border-b border-border/70 bg-muted/20">
            <TableHead className="px-4">{t("Session ID")}</TableHead>
            <TableHead>{t("Proxy")}</TableHead>
            <TableHead>{t("Selected IP")}</TableHead>
            <TableHead>{t("Listen")}</TableHead>
            <TableHead>{t("Created")}</TableHead>
            <TableHead className="pr-4 text-right">{t("Action")}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {sessions.map((session) => {
            const isClosing = closingSessionId === session.session_id;
            return (
              <TableRow key={session.session_id} className="[&_td]:py-3">
                <TableCell className="px-4 font-mono text-xs md:text-sm">
                  {session.session_id}
                </TableCell>
                <TableCell>
                  <div className="space-y-1">
                    <div className="font-medium">{session.proxy_name}</div>
                    <Badge
                      variant="outline"
                      className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.14em]"
                    >
                      {t("port {port}", { port: session.port })}
                    </Badge>
                  </div>
                </TableCell>
                <TableCell className="font-mono text-xs md:text-sm">
                  {session.selected_ip}
                </TableCell>
                <TableCell className="font-mono text-xs md:text-sm">{session.listen}</TableCell>
                <TableCell className="text-xs md:text-sm">
                  {formatTimestamp(locale, t, session.created_at)}
                </TableCell>
                <TableCell className="pr-4 text-right">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => onCloseSession(session.session_id)}
                    disabled={isClosing}
                    className={cn(isClosing && "opacity-70")}
                  >
                    {isClosing ? t("Closing...") : t("Close")}
                  </Button>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
      <ScrollBar orientation="horizontal" />
    </ScrollArea>
  );
}
