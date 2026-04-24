import { PlusCircleIcon, ShieldCheckIcon } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { DataTablePanel } from "@/components/DataTablePanel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { SessionCreateDialog } from "@/features/sessions/components/SessionCreateDialog";
import { SessionNodeSelectDialog } from "@/features/sessions/components/SessionNodeSelectDialog";
import { SessionsTable } from "@/features/sessions/components/SessionsTable";
import { useI18n } from "@/i18n";
import type {
  OpenBatchRequest,
  OpenBatchResponse,
  OpenSessionRequest,
  OpenSessionResponse,
  SearchSessionNodeOptionsRequest,
  SearchSessionOptionsRequest,
  SessionListItem,
  SessionNodeOptionItem,
  SessionOptionItem,
  UpdateSessionNodeRequest,
} from "@/lib/types";

interface SessionsPageProps {
  sessions: SessionListItem[];
  sessionsLoading: boolean;
  openError?: string | null;
  batchError?: string | null;
  switchError?: string | null;
  openResponse?: OpenSessionResponse | null;
  batchResponse?: OpenBatchResponse | null;
  switchedSessionId?: string | null;
  opening: boolean;
  batchOpening: boolean;
  suggestedPort?: number | null;
  closingSessionId?: string | null;
  switchingSessionId?: string | null;
  onOpenSession: (payload: OpenSessionRequest) => void | Promise<void>;
  onOpenBatch: (payload: OpenBatchRequest) => void | Promise<void>;
  onUpdateSessionNode: (
    sessionId: string,
    payload: UpdateSessionNodeRequest,
  ) => void | Promise<void>;
  searchSessionOptions: (
    payload: SearchSessionOptionsRequest,
  ) => Promise<SessionOptionItem[] | undefined>;
  searchSessionNodeOptions: (
    sessionId: string,
    payload: SearchSessionNodeOptionsRequest,
  ) => Promise<SessionNodeOptionItem[] | undefined>;
  onCloseSession: (sessionId: string) => void | Promise<void>;
  onResetCreateState: () => void;
  onResetSwitchState: () => void;
}

export function SessionsPage({
  sessions,
  sessionsLoading,
  openError,
  batchError,
  switchError,
  openResponse,
  batchResponse,
  switchedSessionId,
  opening,
  batchOpening,
  suggestedPort,
  closingSessionId,
  switchingSessionId,
  onOpenSession,
  onOpenBatch,
  onUpdateSessionNode,
  searchSessionOptions,
  searchSessionNodeOptions,
  onCloseSession,
  onResetCreateState,
  onResetSwitchState,
}: SessionsPageProps) {
  const { formatNumber, t } = useI18n();
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);

  const editingSession = useMemo(
    () => sessions.find((session) => session.session_id === editingSessionId) ?? null,
    [editingSessionId, sessions],
  );

  useEffect(() => {
    if (!createDialogOpen) {
      return;
    }
    if (opening || batchOpening) {
      return;
    }
    if (!openResponse && !batchResponse) {
      return;
    }
    setCreateDialogOpen(false);
    onResetCreateState();
  }, [batchOpening, batchResponse, createDialogOpen, onResetCreateState, openResponse, opening]);

  useEffect(() => {
    if (!editingSessionId) {
      return;
    }
    if (switchingSessionId) {
      return;
    }
    if (switchedSessionId !== editingSessionId) {
      return;
    }
    setEditingSessionId(null);
    onResetSwitchState();
  }, [editingSessionId, onResetSwitchState, switchedSessionId, switchingSessionId]);

  useEffect(() => {
    if (editingSessionId && !editingSession && switchingSessionId !== editingSessionId) {
      setEditingSessionId(null);
      onResetSwitchState();
    }
  }, [editingSession, editingSessionId, onResetSwitchState, switchingSessionId]);

  const chips = [
    t(sessions.length === 1 ? "{count} session" : "{count} sessions", {
      count: formatNumber(sessions.length),
    }),
    sessionsLoading ? t("polling now") : t("polling every 5s"),
    switchingSessionId ? t("switch action in flight") : t("switch action idle"),
    closingSessionId ? t("close action in flight") : t("close action idle"),
  ];

  return (
    <div className="space-y-8">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-2">
          <h1 className="text-2xl font-semibold tracking-tight text-foreground">{t("Sessions")}</h1>
          <p className="max-w-3xl text-sm leading-6 text-muted-foreground md:text-[15px]">
            {t(
              "Keep the page focused on the current session inventory. Create new sessions or switch nodes from dialogs when you need them.",
            )}
          </p>
        </div>
        <Button
          onClick={() => {
            onResetCreateState();
            setCreateDialogOpen(true);
          }}
        >
          <PlusCircleIcon className="mr-2 size-4" />
          {t("Create session")}
        </Button>
      </header>

      <DataTablePanel
        eyebrow={t("Current sessions")}
        title={t("Session list")}
        description={t(
          "This list refreshes every five seconds while you stay on the route, so it reflects the backend's current session inventory.",
        )}
        chips={chips}
        actions={
          <Badge
            variant="outline"
            className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
          >
            <ShieldCheckIcon className="mr-1 size-3.5" />
            {t("session control")}
          </Badge>
        }
      >
        <SessionsTable
          closingSessionId={closingSessionId}
          isLoading={sessionsLoading}
          onCloseSession={onCloseSession}
          onEditSession={(sessionId) => {
            onResetSwitchState();
            setEditingSessionId(sessionId);
          }}
          sessions={sessions}
          switchingSessionId={switchingSessionId}
        />
      </DataTablePanel>

      <SessionCreateDialog
        open={createDialogOpen}
        onOpenChange={(nextOpen) => {
          setCreateDialogOpen(nextOpen);
          if (!nextOpen) {
            onResetCreateState();
          }
        }}
        openError={openError}
        batchError={batchError}
        openResponse={openResponse}
        batchResponse={batchResponse}
        opening={opening}
        batchOpening={batchOpening}
        suggestedPort={suggestedPort}
        onOpenSession={onOpenSession}
        onOpenBatch={onOpenBatch}
        searchSessionOptions={searchSessionOptions}
      />

      <SessionNodeSelectDialog
        open={Boolean(editingSession)}
        session={editingSession}
        isPending={Boolean(editingSession && switchingSessionId === editingSession.session_id)}
        error={switchError}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) {
            setEditingSessionId(null);
            onResetSwitchState();
          }
        }}
        onSearch={searchSessionNodeOptions}
        onSubmit={onUpdateSessionNode}
      />
    </div>
  );
}
