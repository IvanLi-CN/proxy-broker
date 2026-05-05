import { LoaderCircleIcon, PencilLineIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  SessionIpNodePicker,
  type SessionIpNodePickerSelection,
} from "@/features/sessions/components/SessionIpNodePicker";
import { useI18n } from "@/i18n";
import { resolveSessionDisplayAddress } from "@/lib/format";
import type {
  SearchSessionIpNodeOptionsRequest,
  SessionIpNodeOptionGroupItem,
  SessionListItem,
  UpdateSessionNodeRequest,
} from "@/lib/types";

interface SessionNodeSelectDialogProps {
  open: boolean;
  session: SessionListItem | null;
  isPending: boolean;
  error?: string | null;
  onOpenChange: (open: boolean) => void;
  onSearch: (
    payload: SearchSessionIpNodeOptionsRequest,
  ) => Promise<SessionIpNodeOptionGroupItem[] | undefined>;
  onSubmit: (sessionId: string, payload: UpdateSessionNodeRequest) => void | Promise<void>;
}

export function SessionNodeSelectDialog({
  open,
  session,
  isPending,
  error,
  onOpenChange,
  onSearch,
  onSubmit,
}: SessionNodeSelectDialogProps) {
  const { t } = useI18n();
  const displayAddress = session ? resolveSessionDisplayAddress(session) : null;
  const [selection, setSelection] = useState<SessionIpNodePickerSelection | null>(null);
  const initializedSessionId = useRef<string | null>(null);

  useEffect(() => {
    if (!open || !session) {
      setSelection(null);
      initializedSessionId.current = null;
      return;
    }
    if (initializedSessionId.current === session.session_id) {
      return;
    }
    initializedSessionId.current = session.session_id;
    setSelection({
      selectedIp: session.selected_ip,
      candidateNodeIds:
        session.candidate_node_ids.length > 0 ? session.candidate_node_ids : [session.node_id],
    });
  }, [open, session]);

  const submitDisabled =
    !session || !selection || selection.candidateNodeIds.length === 0 || isPending;

  const submit = () => {
    if (!session || !selection) {
      return;
    }
    void onSubmit(session.session_id, {
      selected_ip: selection.selectedIp,
      candidate_node_ids: selection.candidateNodeIds,
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[92vh] w-[calc(100vw-2rem)] max-w-[1180px] overflow-y-auto sm:max-w-[1180px]">
        <DialogHeader>
          <DialogTitle>{t("Switch session proxy")}</DialogTitle>
          <DialogDescription>
            {session
              ? t("Pick an IP and candidate nodes for {sessionId}. The listener stays unchanged.", {
                  sessionId: session.session_id,
                })
              : t("Select a session before switching its node.")}
          </DialogDescription>
        </DialogHeader>

        {session ? (
          <div className="rounded-lg border border-border/70 bg-muted/20 px-4 py-3 text-sm">
            <div className="font-medium text-foreground">{session.proxy_name}</div>
            <div className="mt-1 flex flex-wrap gap-2 text-xs text-muted-foreground">
              <span>{t("Session ID: {sessionId}", { sessionId: session.session_id })}</span>
              <span>{t("Address {address}", { address: displayAddress ?? session.listen })}</span>
              <span>{t("Selected IP {ip}", { ip: session.selected_ip })}</span>
            </div>
          </div>
        ) : null}

        {session ? (
          <SessionIpNodePicker
            key={session.session_id}
            mode="single"
            sessionId={session.session_id}
            initialSelectedIp={session.selected_ip}
            initialCandidateNodeIds={
              session.candidate_node_ids.length > 0 ? session.candidate_node_ids : [session.node_id]
            }
            disabled={isPending}
            onSelectionChange={(items) => setSelection(items[0] ?? null)}
            onSearch={onSearch}
          />
        ) : null}

        {error ? (
          <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive">
            {error}
          </div>
        ) : null}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isPending}>
            {t("Cancel")}
          </Button>
          <Button onClick={submit} disabled={submitDisabled}>
            {isPending ? (
              <>
                <LoaderCircleIcon className="mr-2 size-4 animate-spin" />
                {t("Switching proxy...")}
              </>
            ) : (
              <>
                <PencilLineIcon className="mr-2 size-4" />
                {t("Use selected candidates")}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
