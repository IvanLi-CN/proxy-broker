import { BinaryIcon, Rows3Icon, ShieldCheckIcon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { DataTablePanel } from "@/components/DataTablePanel";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { OpenBatchForm } from "@/features/sessions/components/OpenBatchForm";
import { OpenSessionForm } from "@/features/sessions/components/OpenSessionForm";
import { SessionsTable } from "@/features/sessions/components/SessionsTable";
import { useI18n } from "@/i18n";
import type {
  OpenBatchRequest,
  OpenBatchResponse,
  OpenSessionRequest,
  OpenSessionResponse,
  SearchSessionOptionsRequest,
  SessionOptionItem,
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
  suggestedPort?: number | null;
  closingSessionId?: string | null;
  onOpenSession: (payload: OpenSessionRequest) => void | Promise<void>;
  onOpenBatch: (payload: OpenBatchRequest) => void | Promise<void>;
  searchSessionOptions: (
    payload: SearchSessionOptionsRequest,
  ) => Promise<SessionOptionItem[] | undefined>;
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
  suggestedPort,
  closingSessionId,
  onOpenSession,
  onOpenBatch,
  searchSessionOptions,
  onCloseSession,
}: SessionsPageProps) {
  const { formatNumber, t } = useI18n();

  return (
    <div className="space-y-8">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">{t("Sessions")}</h1>
      </header>

      <section className="grid gap-6 xl:grid-cols-[minmax(0,1.02fr)_minmax(0,0.98fr)]">
        <div className="space-y-6">
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
        </div>

        <div className="space-y-4">
          {batchError && !batchResponse ? (
            <ActionResponsePanel
              title={t("Batch open error")}
              description={batchError}
              tone="error"
            />
          ) : null}
          <DataTablePanel
            eyebrow={t("Live listener deck")}
            title={t("Active listeners")}
            description={t(
              "The table refreshes every five seconds while you stay on this route, so the deck mirrors the backend's current session inventory.",
            )}
            chips={[
              t(sessions.length === 1 ? "{count} live row" : "{count} live rows", {
                count: formatNumber(sessions.length),
              }),
              sessionsLoading ? t("polling now") : t("polling every 5s"),
              closingSessionId ? t("close action in flight") : t("close deck idle"),
            ]}
            actions={
              <Badge
                variant="outline"
                className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
              >
                <ShieldCheckIcon className="mr-1 size-3.5" />
                {t("live control")}
              </Badge>
            }
          >
            <SessionsTable
              closingSessionId={closingSessionId}
              isLoading={sessionsLoading}
              onCloseSession={onCloseSession}
              sessions={sessions}
            />
          </DataTablePanel>
        </div>
      </section>
    </div>
  );
}
