import { BinaryIcon, Rows3Icon, ShieldCheckIcon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { DataTablePanel } from "@/components/DataTablePanel";
import { RouteHero } from "@/components/RouteHero";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { WorkflowRail } from "@/components/WorkflowRail";
import { OpenBatchForm } from "@/features/sessions/components/OpenBatchForm";
import { OpenSessionForm } from "@/features/sessions/components/OpenSessionForm";
import { SessionsTable } from "@/features/sessions/components/SessionsTable";
import { useI18n } from "@/i18n";
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
  const { formatNumber, t } = useI18n();
  const newestSession = sessions.reduce<SessionRecord | null>(
    (latest, session) => (latest && latest.created_at > session.created_at ? latest : session),
    null,
  );
  const newestListen = newestSession?.listen ?? null;

  return (
    <div className="space-y-8">
      <RouteHero
        eyebrow={t("Sessions")}
        title={t("Sessions hero title")}
        description={t("Sessions hero description")}
        badges={[
          {
            label: t("{count} live listeners", { count: formatNumber(sessions.length) }),
            tone: sessions.length > 0 ? "positive" : "neutral",
          },
          {
            label: opening || batchOpening ? t("open request active") : t("open deck idle"),
            tone: opening || batchOpening ? "warning" : "positive",
          },
          {
            label: closingSessionId
              ? t("closing {sessionId}", { sessionId: closingSessionId })
              : t("no close in flight"),
            tone: closingSessionId ? "warning" : "neutral",
          },
        ]}
        aside={
          <WorkflowRail
            eyebrow={t("Operating rule")}
            title={t("Treat listeners like inventory")}
            steps={[
              {
                title: t("Open deliberately"),
                description: t(
                  "Prefer one explicit listener when the target edge is already known.",
                ),
              },
              {
                title: t("Use batch when rollback matters"),
                description: t("Stage multiple rows only when they form one logical operation."),
              },
              {
                title: t("Close stale listeners quickly"),
                description: t(
                  "Keep the live list small so ports and edge ownership stay obvious.",
                ),
              },
            ]}
          />
        }
      />

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
              />
            </TabsContent>
            <TabsContent value="batch" className="mt-0">
              <OpenBatchForm
                error={batchError}
                isPending={batchOpening}
                onSubmit={onOpenBatch}
                response={batchResponse}
              />
            </TabsContent>
          </Tabs>

          <Card className="border-border/70 bg-card/96 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
            <CardHeader className="space-y-3 border-b border-border/70 pb-5">
              <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
                {t("Control note")}
              </div>
              <CardTitle className="text-xl tracking-tight">
                {t("Listener hygiene matters")}
              </CardTitle>
              <CardDescription className="text-sm leading-6 text-muted-foreground">
                {t(
                  "Ports and proxy edges are operational resources. The cleaner this deck stays, the easier it is to understand what the profile is actually doing.",
                )}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4 pt-6">
              <div className="rounded-2xl border border-border/70 bg-background/80 p-4 text-sm leading-6 text-muted-foreground">
                {t(
                  "Use the single-session form when you need one deterministic listener quickly. Switch to batch only when several ports must succeed or fail together.",
                )}
              </div>
              {newestListen ? (
                <Badge
                  variant="outline"
                  className="rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.16em]"
                >
                  {t("newest listen {listen}", { listen: newestListen })}
                </Badge>
              ) : null}
            </CardContent>
          </Card>
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
