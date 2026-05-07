import { PlusCircleIcon, RotateCcwIcon, XCircleIcon } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { DataTablePanel } from "@/components/DataTablePanel";
import { Button } from "@/components/ui/button";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { SessionCreateDialog } from "@/features/sessions/components/SessionCreateDialog";
import { SessionNodeSelectDialog } from "@/features/sessions/components/SessionNodeSelectDialog";
import { SessionsTable } from "@/features/sessions/components/SessionsTable";
import {
  type SessionCopyAddressFormat,
  useSessionCopyAddressFormat,
} from "@/features/sessions/hooks/use-session-copy-address-format";
import type { ProxyNodeLiveState } from "@/hooks/use-proxy-operation-events";
import { useI18n } from "@/i18n";
import type {
  OpenBatchByIpRequest,
  OpenBatchResponse,
  OpenSessionByIpRequest,
  OpenSessionResponse,
  SearchSessionIpNodeOptionsRequest,
  SearchSessionNodeOptionsRequest,
  SessionIpNodeOptionGroupItem,
  SessionListItem,
  SessionNodeOptionItem,
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
  onOpenSession: (payload: OpenSessionByIpRequest) => void | Promise<void>;
  onOpenBatch: (payload: OpenBatchByIpRequest) => void | Promise<void>;
  onUpdateSessionNode: (
    sessionId: string,
    payload: UpdateSessionNodeRequest,
  ) => void | Promise<void>;
  searchSessionIpNodeOptions: (
    payload: SearchSessionIpNodeOptionsRequest,
  ) => Promise<SessionIpNodeOptionGroupItem[] | undefined>;
  onProbeSessionNodes: (nodeIds: string[]) => void | Promise<void>;
  probingNodeIds?: string[];
  liveNodeStates?: Record<string, ProxyNodeLiveState>;
  probeNodeStates?: Record<string, ProxyNodeLiveState>;
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
  searchSessionIpNodeOptions,
  onProbeSessionNodes,
  probingNodeIds = [],
  liveNodeStates = {},
  probeNodeStates = {},
  searchSessionNodeOptions,
  onCloseSession,
  onResetCreateState,
  onResetSwitchState,
}: SessionsPageProps) {
  const { formatNumber, t } = useI18n();
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [listenCopyFormat, setListenCopyFormat] = useSessionCopyAddressFormat();
  const [pendingCloseSessionIds, setPendingCloseSessionIds] = useState<string[]>([]);
  const [hiddenCloseSessionIds, setHiddenCloseSessionIds] = useState<string[]>([]);
  const [selectedSessionIds, setSelectedSessionIds] = useState<string[]>([]);
  const closeCountdownTimersRef = useRef<Map<string, number>>(new Map());

  const editingSession = useMemo(
    () => sessions.find((session) => session.session_id === editingSessionId) ?? null,
    [editingSessionId, sessions],
  );
  const visibleSessions = useMemo(
    () => sessions.filter((session) => !hiddenCloseSessionIds.includes(session.session_id)),
    [hiddenCloseSessionIds, sessions],
  );
  const visibleSessionIds = useMemo(
    () => visibleSessions.map((session) => session.session_id),
    [visibleSessions],
  );
  const visibleSessionIdSet = useMemo(() => new Set(visibleSessionIds), [visibleSessionIds]);
  const copyAddressOptions = useMemo(
    () =>
      [
        {
          value: "socks_url",
          label: t("SOCKS URI"),
        },
        {
          value: "http_url",
          label: t("HTTP URI"),
        },
        {
          value: "host_port",
          label: t("Host:port"),
        },
      ] satisfies Array<{
        value: SessionCopyAddressFormat;
        label: string;
      }>,
    [t],
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

  useEffect(() => {
    const currentSessionIds = new Set(sessions.map((session) => session.session_id));

    setPendingCloseSessionIds((previous) =>
      previous.filter((sessionId) => {
        if (currentSessionIds.has(sessionId)) {
          return true;
        }

        const timerId = closeCountdownTimersRef.current.get(sessionId);
        if (timerId !== undefined) {
          window.clearTimeout(timerId);
          closeCountdownTimersRef.current.delete(sessionId);
        }
        return false;
      }),
    );

    setHiddenCloseSessionIds((previous) =>
      previous.filter((sessionId) => currentSessionIds.has(sessionId)),
    );
  }, [sessions]);

  useEffect(() => {
    setSelectedSessionIds((previous) =>
      previous.filter((sessionId) => visibleSessionIdSet.has(sessionId)),
    );
  }, [visibleSessionIdSet]);

  useEffect(
    () => () => {
      for (const timerId of closeCountdownTimersRef.current.values()) {
        window.clearTimeout(timerId);
      }
      closeCountdownTimersRef.current.clear();
    },
    [],
  );

  const beginCloseCountdown = (sessionId: string) => {
    if (closeCountdownTimersRef.current.has(sessionId)) {
      return;
    }

    setPendingCloseSessionIds((previous) =>
      previous.includes(sessionId) ? previous : [...previous, sessionId],
    );

    const timerId = window.setTimeout(() => {
      closeCountdownTimersRef.current.delete(sessionId);
      setPendingCloseSessionIds((previous) => previous.filter((id) => id !== sessionId));
      setHiddenCloseSessionIds((previous) =>
        previous.includes(sessionId) ? previous : [...previous, sessionId],
      );

      void Promise.resolve(onCloseSession(sessionId)).catch(() => {
        setHiddenCloseSessionIds((previous) => previous.filter((id) => id !== sessionId));
      });
    }, 10_000);

    closeCountdownTimersRef.current.set(sessionId, timerId);
  };

  const undoCloseCountdown = (sessionId: string) => {
    const timerId = closeCountdownTimersRef.current.get(sessionId);
    if (timerId !== undefined) {
      window.clearTimeout(timerId);
      closeCountdownTimersRef.current.delete(sessionId);
    }

    setPendingCloseSessionIds((previous) => previous.filter((id) => id !== sessionId));
  };

  const selectedVisibleSessionIds = selectedSessionIds.filter((sessionId) =>
    visibleSessionIdSet.has(sessionId),
  );
  const selectedPendingSessionIds = selectedVisibleSessionIds.filter((sessionId) =>
    pendingCloseSessionIds.includes(sessionId),
  );
  const selectedClosableSessionIds = selectedVisibleSessionIds.filter(
    (sessionId) =>
      !pendingCloseSessionIds.includes(sessionId) &&
      closingSessionId !== sessionId &&
      switchingSessionId !== sessionId,
  );

  const chips = [
    t(visibleSessions.length === 1 ? "{count} session" : "{count} sessions", {
      count: formatNumber(visibleSessions.length),
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
        toolbar={
          <div className="flex w-full max-w-[420px] items-center justify-end gap-3 lg:min-w-[360px]">
            <div className="shrink-0 text-[11px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
              {t("Copy format")}
            </div>
            <ToggleGroup
              type="single"
              value={listenCopyFormat}
              onValueChange={(value) => {
                if (value === "socks_url" || value === "http_url" || value === "host_port") {
                  setListenCopyFormat(value);
                }
              }}
              aria-label={t("Copy format")}
              className="min-w-0 flex-1 rounded-full border border-border/70 bg-background/80 p-1"
            >
              {copyAddressOptions.map((option) => (
                <ToggleGroupItem
                  key={option.value}
                  value={option.value}
                  aria-label={option.label}
                  className="flex-1 rounded-full border border-transparent px-3 py-2 text-xs font-semibold text-muted-foreground shadow-none transition-[color,background-color,border-color,box-shadow] hover:bg-muted/60 hover:text-foreground data-[state=on]:border-primary/25 data-[state=on]:bg-primary data-[state=on]:text-primary-foreground data-[state=on]:shadow-[0_10px_24px_-16px_hsl(var(--primary)/0.9),inset_0_0_0_1px_hsl(var(--background)/0.14)] hover:data-[state=on]:bg-primary/92 sm:text-[13px]"
                >
                  {option.label}
                </ToggleGroupItem>
              ))}
            </ToggleGroup>
          </div>
        }
      >
        {visibleSessions.length > 0 ? (
          <div className="mb-3 flex flex-wrap items-center justify-between gap-3 rounded-[16px] border border-dashed border-border/70 bg-muted/10 px-3 py-2 text-xs text-muted-foreground">
            <span>
              {selectedVisibleSessionIds.length > 0
                ? t("Selected {count} sessions", {
                    count: formatNumber(selectedVisibleSessionIds.length),
                  })
                : t("Select one or more sessions to run batch operations.")}
            </span>
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                size="sm"
                disabled={selectedPendingSessionIds.length === 0}
                onClick={() => {
                  for (const sessionId of selectedPendingSessionIds) {
                    undoCloseCountdown(sessionId);
                  }
                }}
              >
                <RotateCcwIcon className="size-3.5" />
                {t("Undo selected")}
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={selectedClosableSessionIds.length === 0}
                onClick={() => {
                  for (const sessionId of selectedClosableSessionIds) {
                    beginCloseCountdown(sessionId);
                  }
                }}
              >
                <XCircleIcon className="size-3.5" />
                {t("Close selected")}
              </Button>
            </div>
          </div>
        ) : null}
        <SessionsTable
          closingSessionId={closingSessionId}
          isLoading={sessionsLoading}
          listenCopyFormat={listenCopyFormat}
          onCloseSession={beginCloseCountdown}
          onEditSession={(sessionId) => {
            onResetSwitchState();
            setEditingSessionId(sessionId);
          }}
          onUndoCloseSession={undoCloseCountdown}
          pendingCloseSessionIds={pendingCloseSessionIds}
          selectedSessionIds={selectedSessionIds}
          onSelectedSessionIdsChange={setSelectedSessionIds}
          sessions={visibleSessions}
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
        searchIpNodeOptions={searchSessionIpNodeOptions}
      />

      <SessionNodeSelectDialog
        open={Boolean(editingSession)}
        session={editingSession}
        isPending={Boolean(editingSession && switchingSessionId === editingSession.session_id)}
        error={switchError}
        probingNodeIds={probingNodeIds}
        liveNodeStates={liveNodeStates}
        probeNodeStates={probeNodeStates}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) {
            setEditingSessionId(null);
            onResetSwitchState();
          }
        }}
        onSearch={searchSessionNodeOptions}
        onSubmit={onUpdateSessionNode}
        onProbeNodes={onProbeSessionNodes}
      />
    </div>
  );
}
