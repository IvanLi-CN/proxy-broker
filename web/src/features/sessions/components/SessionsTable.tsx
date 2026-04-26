import { CopyIcon, LoaderCircleIcon, PencilLineIcon, PlugZapIcon } from "lucide-react";
import { toast } from "sonner";

import { EmptyPanel } from "@/components/EmptyPanel";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { SessionCopyAddressFormat } from "@/features/sessions/hooks/use-session-copy-address-format";
import { useTableRangeSelection } from "@/hooks/use-table-range-selection";
import { useI18n } from "@/i18n";
import {
  buildProxyAddressFromDisplayAddress,
  formatCountryName,
  formatGeoLabel,
  formatTimestamp,
  resolveSessionDisplayAddress,
} from "@/lib/format";
import type { SessionListItem } from "@/lib/types";
import { cn } from "@/lib/utils";

function buildSessionGeoSummary(locale: "zh-CN" | "en-US", session: SessionListItem) {
  const parts = [
    formatCountryName(locale, session.country_code, session.country_name),
    formatGeoLabel(locale, session.region_name),
    formatGeoLabel(locale, session.city),
  ].filter(Boolean);
  return Array.from(new Set(parts)).join(" / ");
}

async function copyTextToClipboard(value: string) {
  if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(value);
    return;
  }

  if (typeof document === "undefined") {
    throw new Error("clipboard unavailable");
  }

  const textArea = document.createElement("textarea");
  textArea.value = value;
  textArea.setAttribute("readonly", "");
  textArea.style.position = "absolute";
  textArea.style.left = "-9999px";
  document.body.appendChild(textArea);
  textArea.select();
  const copied = document.execCommand?.("copy");
  document.body.removeChild(textArea);
  if (!copied) {
    throw new Error("clipboard unavailable");
  }
}

interface SessionsTableProps {
  sessions: SessionListItem[];
  listenCopyFormat: SessionCopyAddressFormat;
  isLoading?: boolean;
  pendingCloseSessionIds?: string[];
  closingSessionId?: string | null;
  switchingSessionId?: string | null;
  selectedSessionIds: string[];
  onSelectedSessionIdsChange: (sessionIds: string[]) => void;
  onEditSession: (sessionId: string) => void;
  onUndoCloseSession: (sessionId: string) => void;
  onCloseSession: (sessionId: string) => void;
}

export function SessionsTable({
  sessions,
  listenCopyFormat,
  isLoading,
  pendingCloseSessionIds = [],
  closingSessionId,
  switchingSessionId,
  selectedSessionIds,
  onSelectedSessionIdsChange,
  onEditSession,
  onUndoCloseSession,
  onCloseSession,
}: SessionsTableProps) {
  const { locale, t } = useI18n();
  const selection = useTableRangeSelection({
    itemIds: sessions.map((session) => session.session_id),
    selectedIds: selectedSessionIds,
    onSelectedIdsChange: onSelectedSessionIdsChange,
  });

  const handleCopyAddress = async (session: SessionListItem) => {
    const displayAddress = resolveSessionDisplayAddress(session);
    if (!displayAddress) {
      toast.error(t("Could not copy proxy address"));
      return;
    }

    try {
      await copyTextToClipboard(
        buildProxyAddressFromDisplayAddress(displayAddress, listenCopyFormat),
      );
      toast.success(t("Copied proxy address"));
    } catch {
      toast.error(t("Could not copy proxy address"));
    }
  };

  if (isLoading && sessions.length === 0) {
    return (
      <EmptyPanel
        title={t("Loading sessions")}
        description={t("Polling the backend for sessions on this profile.")}
        icon={LoaderCircleIcon}
        hint={t("The current session list appears here as soon as the first response lands.")}
      />
    );
  }

  if (sessions.length === 0) {
    return (
      <EmptyPanel
        title={t("No sessions yet")}
        description={t("Create one session or a batch from the dialog to populate this list.")}
        icon={PlugZapIcon}
      />
    );
  }

  return (
    <ScrollArea className="rounded-[28px] border border-border/70 bg-card/90 shadow-sm">
      <Table className="min-w-[920px]">
        <TableHeader>
          <TableRow className="border-b border-border/70 bg-muted/20">
            <TableHead className="w-10 px-4">
              <Checkbox
                {...selection.selectAllCheckboxProps}
                aria-label={t("Select all visible sessions")}
              />
            </TableHead>
            <TableHead className="px-4">{t("Session ID")}</TableHead>
            <TableHead>{t("Proxy")}</TableHead>
            <TableHead>{t("Selected IP")}</TableHead>
            <TableHead>{t("Proxy address")}</TableHead>
            <TableHead>{t("Created")}</TableHead>
            <TableHead className="pr-4 text-right">{t("Action")}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {sessions.map((session) => {
            const isPendingClose = pendingCloseSessionIds.includes(session.session_id);
            const isClosing = closingSessionId === session.session_id;
            const isSwitching = switchingSessionId === session.session_id;
            const isDimmed = isPendingClose || isClosing;
            const geoSummary = buildSessionGeoSummary(locale, session);
            const displayAddress = resolveSessionDisplayAddress(session);
            return (
              <TableRow
                key={session.session_id}
                data-close-state={isPendingClose ? "pending" : isClosing ? "closing" : "idle"}
                data-state={selectedSessionIds.includes(session.session_id) ? "selected" : "idle"}
                className={cn("[&_td]:py-3", isDimmed && "bg-muted/30 text-muted-foreground")}
              >
                <TableCell
                  className="touch-none px-4 align-middle"
                  {...selection.getSelectionCellProps(session.session_id)}
                >
                  <Checkbox
                    {...selection.getCheckboxProps(session.session_id)}
                    aria-label={t("Select session {sessionId}", {
                      sessionId: session.session_id,
                    })}
                  />
                </TableCell>
                <TableCell className="px-4 font-mono text-xs md:text-sm">
                  {session.session_id}
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-2">
                    <div className="font-medium">{session.proxy_name}</div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="size-7 rounded-full"
                      aria-label={t("Edit proxy for {sessionId}", {
                        sessionId: session.session_id,
                      })}
                      onClick={() => onEditSession(session.session_id)}
                      disabled={isSwitching || isClosing || isPendingClose}
                    >
                      {isSwitching ? (
                        <LoaderCircleIcon className="size-3.5 animate-spin" />
                      ) : (
                        <PencilLineIcon className="size-3.5" />
                      )}
                    </Button>
                  </div>
                </TableCell>
                <TableCell>
                  <div className="space-y-1">
                    <div className="font-mono text-xs md:text-sm">{session.selected_ip}</div>
                    {geoSummary ? (
                      <div className="text-xs text-muted-foreground">{geoSummary}</div>
                    ) : null}
                  </div>
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-1.5">
                    <span className="font-mono text-xs md:text-sm">{displayAddress}</span>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="size-7 rounded-full"
                      aria-label={t("Copy proxy address for {sessionId}", {
                        sessionId: session.session_id,
                      })}
                      onClick={() => {
                        void handleCopyAddress(session);
                      }}
                      disabled={isDimmed || isSwitching || !displayAddress}
                    >
                      <CopyIcon className="size-3.5" />
                    </Button>
                  </div>
                </TableCell>
                <TableCell className="text-xs md:text-sm">
                  {formatTimestamp(locale, t, session.created_at)}
                </TableCell>
                <TableCell className="pr-4 text-right">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() =>
                      isPendingClose
                        ? onUndoCloseSession(session.session_id)
                        : onCloseSession(session.session_id)
                    }
                    disabled={isClosing || isSwitching}
                    className={cn(
                      (isClosing || isSwitching) && "opacity-70",
                      isPendingClose && "text-foreground hover:text-foreground",
                    )}
                  >
                    {isPendingClose ? t("Undo") : isClosing ? t("Closing...") : t("Close")}
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
