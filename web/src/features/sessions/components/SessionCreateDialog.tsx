import { BinaryIcon, Rows3Icon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { OpenBatchForm } from "@/features/sessions/components/OpenBatchForm";
import { OpenSessionForm } from "@/features/sessions/components/OpenSessionForm";
import { useI18n } from "@/i18n";
import type {
  OpenBatchRequest,
  OpenBatchResponse,
  OpenSessionRequest,
  OpenSessionResponse,
  SearchSessionOptionsRequest,
  SessionOptionItem,
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
  onOpenSession: (payload: OpenSessionRequest) => void | Promise<void>;
  onOpenBatch: (payload: OpenBatchRequest) => void | Promise<void>;
  searchSessionOptions: (
    payload: SearchSessionOptionsRequest,
  ) => Promise<SessionOptionItem[] | undefined>;
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
  searchSessionOptions,
}: SessionCreateDialogProps) {
  const { t } = useI18n();

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-5xl">
        <DialogHeader>
          <DialogTitle>{t("Create session")}</DialogTitle>
          <DialogDescription>
            {t(
              "Open a new session from one dialog. Keep single and batch creation together, but leave the list as the default surface.",
            )}
          </DialogDescription>
        </DialogHeader>

        {batchError && !batchResponse ? (
          <ActionResponsePanel
            title={t("Batch create failed")}
            description={batchError}
            tone="error"
          />
        ) : null}

        <Tabs defaultValue="single" className="space-y-4">
          <TabsList className="grid w-full grid-cols-2 rounded-2xl border border-border/70 bg-card/80 p-1">
            <TabsTrigger value="single" className="gap-2 rounded-xl">
              <BinaryIcon className="size-4" />
              {t("Single session")}
            </TabsTrigger>
            <TabsTrigger value="batch" className="gap-2 rounded-xl">
              <Rows3Icon className="size-4" />
              {t("Batch open")}
            </TabsTrigger>
          </TabsList>
          <TabsContent value="single" className="mt-0">
            <OpenSessionForm
              error={openError}
              isPending={opening}
              onSubmit={onOpenSession}
              response={openResponse}
              searchOptions={searchSessionOptions}
              suggestedPort={suggestedPort}
            />
          </TabsContent>
          <TabsContent value="batch" className="mt-0">
            <OpenBatchForm
              error={batchError}
              isPending={batchOpening}
              onSubmit={onOpenBatch}
              response={batchResponse}
              searchOptions={searchSessionOptions}
              suggestedPort={suggestedPort}
            />
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
