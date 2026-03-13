import { BinaryIcon, Rows3Icon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { OpenBatchForm } from "@/features/sessions/components/OpenBatchForm";
import { OpenSessionForm } from "@/features/sessions/components/OpenSessionForm";
import { SessionsTable } from "@/features/sessions/components/SessionsTable";
import type {
  OpenBatchRequest,
  OpenBatchResponse,
  OpenSessionRequest,
  OpenSessionResponse,
  SessionRecord,
} from "@/lib/types";

interface SessionsPageProps {
  sessions: SessionRecord[];
  sessionsLoading: boolean;
  openError?: string | null;
  batchError?: string | null;
  openResponse?: OpenSessionResponse | null;
  batchResponse?: OpenBatchResponse | null;
  opening: boolean;
  batchOpening: boolean;
  closingSessionId?: string | null;
  onOpenSession: (payload: OpenSessionRequest) => void | Promise<void>;
  onOpenBatch: (payload: OpenBatchRequest) => void | Promise<void>;
  onCloseSession: (sessionId: string) => void | Promise<void>;
}

export function SessionsPage({
  sessions,
  sessionsLoading,
  openError,
  batchError,
  openResponse,
  batchResponse,
  opening,
  batchOpening,
  closingSessionId,
  onOpenSession,
  onOpenBatch,
  onCloseSession,
}: SessionsPageProps) {
  return (
    <div className="space-y-6">
      <section className="space-y-3">
        <div className="text-xs font-semibold uppercase tracking-[0.28em] text-primary">
          Session orchestration
        </div>
        <h1 className="text-3xl font-semibold tracking-tight text-foreground md:text-4xl">
          Open single listeners or transactional batches from the same control surface.
        </h1>
        <p className="max-w-3xl text-sm leading-7 text-muted-foreground md:text-base">
          Use the single-session form when you need one deterministic listener quickly, or switch to
          the batch builder to request several ports at once and let the backend enforce rollback.
        </p>
      </section>

      <section className="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
        <div className="space-y-6">
          <Tabs defaultValue="single">
            <TabsList>
              <TabsTrigger value="single">
                <BinaryIcon className="size-4" />
                Single session
              </TabsTrigger>
              <TabsTrigger value="batch">
                <Rows3Icon className="size-4" />
                Batch open
              </TabsTrigger>
            </TabsList>
            <TabsContent value="single">
              <OpenSessionForm
                error={openError}
                isPending={opening}
                onSubmit={onOpenSession}
                response={openResponse}
              />
            </TabsContent>
            <TabsContent value="batch">
              <OpenBatchForm
                error={batchError}
                isPending={batchOpening}
                onSubmit={onOpenBatch}
                response={batchResponse}
              />
            </TabsContent>
          </Tabs>
        </div>
        <div className="space-y-4 rounded-2xl border border-border/70 bg-background/70 p-4">
          <div>
            <div className="text-sm font-semibold text-foreground">Active listeners</div>
            <div className="text-xs text-muted-foreground">
              The table refreshes every 5 seconds while you stay on this page.
            </div>
          </div>
          {batchError && !batchResponse ? (
            <ActionResponsePanel title="Batch open error" description={batchError} tone="error" />
          ) : null}
          <SessionsTable
            closingSessionId={closingSessionId}
            isLoading={sessionsLoading}
            onCloseSession={onCloseSession}
            sessions={sessions}
          />
        </div>
      </section>
    </div>
  );
}
