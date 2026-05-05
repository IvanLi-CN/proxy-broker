import { LoaderCircleIcon, PlusIcon } from "lucide-react";
import { useState } from "react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
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
import type {
  OpenBatchByIpRequest,
  OpenBatchResponse,
  OpenSessionByIpRequest,
  OpenSessionResponse,
  SearchSessionIpNodeOptionsRequest,
  SessionIpNodeOptionGroupItem,
} from "@/lib/types";

interface SessionCreateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  openError?: string | null;
  batchError?: string | null;
  openResponse?: OpenSessionResponse | null;
  batchResponse?: OpenBatchResponse | null;
  opening: boolean;
  batchOpening: boolean;
  suggestedPort?: number | null;
  onOpenSession: (payload: OpenSessionByIpRequest) => void | Promise<void>;
  onOpenBatch: (payload: OpenBatchByIpRequest) => void | Promise<void>;
  searchIpNodeOptions: (
    payload: SearchSessionIpNodeOptionsRequest,
  ) => Promise<SessionIpNodeOptionGroupItem[] | undefined>;
}

export function SessionCreateDialog({
  open,
  onOpenChange,
  openError,
  batchError,
  openResponse,
  batchResponse,
  opening,
  batchOpening,
  suggestedPort,
  onOpenSession,
  onOpenBatch,
  searchIpNodeOptions,
}: SessionCreateDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {open ? (
        <SessionCreateDialogContent
          openError={openError}
          batchError={batchError}
          openResponse={openResponse}
          batchResponse={batchResponse}
          opening={opening}
          batchOpening={batchOpening}
          suggestedPort={suggestedPort}
          onOpenChange={onOpenChange}
          onOpenSession={onOpenSession}
          onOpenBatch={onOpenBatch}
          searchIpNodeOptions={searchIpNodeOptions}
        />
      ) : null}
    </Dialog>
  );
}

function SessionCreateDialogContent({
  openError,
  batchError,
  openResponse,
  batchResponse,
  opening,
  batchOpening,
  suggestedPort,
  onOpenChange,
  onOpenSession,
  onOpenBatch,
  searchIpNodeOptions,
}: Omit<SessionCreateDialogProps, "open">) {
  const { t } = useI18n();
  const [selections, setSelections] = useState<SessionIpNodePickerSelection[]>([]);
  const pending = opening || batchOpening;
  const error = batchError || openError;
  const canSubmit =
    selections.length > 0 && selections.every((item) => item.candidateNodeIds.length > 0);

  const submit = () => {
    const requests = selections.map((selection) => ({
      selected_ip: selection.selectedIp,
      candidate_node_ids: selection.candidateNodeIds,
      desired_port: selections.length === 1 ? suggestedPort : undefined,
    }));
    if (requests.length === 1 && requests[0]) {
      void onOpenSession(requests[0]);
      return;
    }
    void onOpenBatch({ requests });
  };

  return (
    <DialogContent className="max-h-[92vh] w-[calc(100vw-2rem)] max-w-[1180px] overflow-y-auto sm:max-w-[1180px]">
      <DialogHeader>
        <DialogTitle>{t("Create session")}</DialogTitle>
        <DialogDescription>
          {t("Choose one or more IPs, then keep the candidate nodes that may serve each session.")}
        </DialogDescription>
      </DialogHeader>

      {error && !openResponse && !batchResponse ? (
        <ActionResponsePanel title={t("Create session failed")} description={error} tone="error" />
      ) : null}

      <SessionIpNodePicker
        mode="multiple"
        disabled={pending}
        onSelectionChange={setSelections}
        onSearch={searchIpNodeOptions}
      />

      <DialogFooter className="items-center gap-3 sm:justify-between">
        <div className="text-xs text-muted-foreground">
          {t("{count} IPs selected", { count: selections.length })}
        </div>
        <div className="flex gap-2">
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
            {t("Cancel")}
          </Button>
          <Button onClick={submit} disabled={!canSubmit || pending}>
            {pending ? (
              <>
                <LoaderCircleIcon className="mr-2 size-4 animate-spin" />
                {t("Creating sessions...")}
              </>
            ) : (
              <>
                <PlusIcon className="mr-2 size-4" />
                {selections.length > 1 ? t("Create sessions") : t("Create session")}
              </>
            )}
          </Button>
        </div>
      </DialogFooter>
    </DialogContent>
  );
}
