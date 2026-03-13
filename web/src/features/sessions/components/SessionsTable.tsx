import { LoaderCircleIcon, PlugZapIcon } from "lucide-react";

import { EmptyPanel } from "@/components/EmptyPanel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { formatTimestamp } from "@/lib/format";
import type { SessionRecord } from "@/lib/types";

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
  if (isLoading && sessions.length === 0) {
    return (
      <EmptyPanel
        title="Loading sessions"
        description="Polling the backend for active listeners on this profile."
        icon={LoaderCircleIcon}
      />
    );
  }

  if (sessions.length === 0) {
    return (
      <EmptyPanel
        title="No active sessions"
        description="Open a single session or a batch to populate this table."
        icon={PlugZapIcon}
      />
    );
  }

  return (
    <ScrollArea className="rounded-2xl border border-border/70 bg-card/90 shadow-sm">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Session ID</TableHead>
            <TableHead>Proxy</TableHead>
            <TableHead>Selected IP</TableHead>
            <TableHead>Listen</TableHead>
            <TableHead>Created</TableHead>
            <TableHead className="text-right">Action</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {sessions.map((session) => (
            <TableRow key={session.session_id}>
              <TableCell className="font-mono text-xs md:text-sm">{session.session_id}</TableCell>
              <TableCell>
                <div className="space-y-1">
                  <div className="font-medium">{session.proxy_name}</div>
                  <Badge variant="secondary" className="font-mono text-[11px]">
                    {session.port}
                  </Badge>
                </div>
              </TableCell>
              <TableCell className="font-mono text-xs md:text-sm">{session.selected_ip}</TableCell>
              <TableCell className="font-mono text-xs md:text-sm">{session.listen}</TableCell>
              <TableCell>{formatTimestamp(session.created_at)}</TableCell>
              <TableCell className="text-right">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onCloseSession(session.session_id)}
                  disabled={closingSessionId === session.session_id}
                >
                  {closingSessionId === session.session_id ? "Closing..." : "Close"}
                </Button>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </ScrollArea>
  );
}
