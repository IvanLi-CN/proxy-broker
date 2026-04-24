import { LoaderCircleIcon, PencilLineIcon } from "lucide-react";
import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useI18n } from "@/i18n";
import {
  formatCountryName,
  formatGeoLabel,
  formatLatency,
  formatListenEndpoint,
  formatTimestamp,
} from "@/lib/format";
import type {
  SessionListItem,
  SessionNodeOptionItem,
  SessionNodeSortMode,
  UpdateSessionNodeRequest,
} from "@/lib/types";
import { cn } from "@/lib/utils";

interface SessionNodeSelectDialogProps {
  open: boolean;
  session: SessionListItem | null;
  isPending: boolean;
  error?: string | null;
  onOpenChange: (open: boolean) => void;
  onSearch: (
    sessionId: string,
    payload: { query?: string; sort_mode: SessionNodeSortMode; limit?: number },
  ) => Promise<SessionNodeOptionItem[] | undefined>;
  onSubmit: (sessionId: string, payload: UpdateSessionNodeRequest) => void | Promise<void>;
}

function buildGeoSummary(locale: "zh-CN" | "en-US", item: SessionNodeOptionItem) {
  const geo = [
    formatCountryName(locale, item.country_code, item.country_name),
    formatGeoLabel(locale, item.region_name),
    formatGeoLabel(locale, item.city),
  ].filter(Boolean);
  return geo.join(" / ");
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
  const { locale, t } = useI18n();
  const listenEndpoint = session ? formatListenEndpoint(session.listen, session.port) : null;
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const [sortMode, setSortMode] = useState<SessionNodeSortMode>("session_recent");
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [items, setItems] = useState<SessionNodeOptionItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const requestVersion = useRef(0);

  useEffect(() => {
    if (!open || !session) {
      return;
    }
    setSelectedNodeId(session.node_id);
  }, [open, session]);

  useEffect(() => {
    if (!open || !session) {
      return;
    }

    const currentVersion = ++requestVersion.current;
    setLoading(true);
    setLoadError(null);

    void onSearch(session.session_id, {
      query: deferredQuery.trim() || undefined,
      sort_mode: sortMode,
      limit: 50,
    })
      .then((nextItems) => {
        if (requestVersion.current !== currentVersion) {
          return;
        }
        setItems(nextItems ?? []);
      })
      .catch(() => {
        if (requestVersion.current !== currentVersion) {
          return;
        }
        setItems([]);
        setLoadError(t("Could not load node options"));
      })
      .finally(() => {
        if (requestVersion.current === currentVersion) {
          setLoading(false);
        }
      });
  }, [deferredQuery, onSearch, open, session, sortMode, t]);

  const selectedItem = useMemo(
    () => items.find((item) => item.node_id === selectedNodeId) ?? null,
    [items, selectedNodeId],
  );

  const handleOpenChange = (nextOpen: boolean) => {
    onOpenChange(nextOpen);
    if (!nextOpen) {
      setQuery("");
      setSortMode("session_recent");
      setItems([]);
      setLoadError(null);
      setSelectedNodeId(session?.node_id ?? null);
    }
  };

  const submitDisabled =
    !session || !selectedNodeId || selectedNodeId === session.node_id || isPending;

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t("Switch session proxy")}</DialogTitle>
          <DialogDescription>
            {session
              ? t(
                  "Pick a new node for {sessionId}. The session keeps the same listener and port.",
                  {
                    sessionId: session.session_id,
                  },
                )
              : t("Select a session before switching its node.")}
          </DialogDescription>
        </DialogHeader>

        {session ? (
          <div className="rounded-2xl border border-border/70 bg-muted/20 px-4 py-3 text-sm">
            <div className="font-medium text-foreground">{session.proxy_name}</div>
            <div className="mt-1 flex flex-wrap gap-2 text-xs text-muted-foreground">
              <span>{t("Session ID: {sessionId}", { sessionId: session.session_id })}</span>
              <span>{t("Listen {listen}", { listen: listenEndpoint ?? session.listen })}</span>
              <span>{t("Selected IP {ip}", { ip: session.selected_ip })}</span>
            </div>
          </div>
        ) : null}

        <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_220px] md:items-end">
          <div className="space-y-2">
            <Label htmlFor="session-node-query">{t("Filter nodes")}</Label>
            <Input
              id="session-node-query"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("Search by node, source, IP, or location")}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="session-node-sort">{t("Sort by")}</Label>
            <Select
              value={sortMode}
              onValueChange={(value) => setSortMode(value as SessionNodeSortMode)}
            >
              <SelectTrigger id="session-node-sort" className="w-full bg-card">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="session_recent">{t("Current session last used")}</SelectItem>
                <SelectItem value="profile_recent">{t("Current profile last used")}</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div className="max-h-[420px] space-y-3 overflow-y-auto pr-1">
          {loading ? (
            <div className="flex items-center justify-center gap-2 rounded-2xl border border-dashed border-border/80 px-4 py-12 text-sm text-muted-foreground">
              <LoaderCircleIcon className="size-4 animate-spin" />
              {t("Loading node options…")}
            </div>
          ) : null}
          {!loading && loadError ? (
            <div className="rounded-2xl border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive">
              {loadError}
            </div>
          ) : null}
          {!loading && !loadError && items.length === 0 ? (
            <div className="rounded-2xl border border-dashed border-border/80 px-4 py-12 text-sm text-muted-foreground">
              {t("No matching nodes")}
            </div>
          ) : null}
          {!loading && !loadError
            ? items.map((item) => {
                const selected = item.node_id === selectedNodeId;
                const current = item.node_id === session?.node_id;
                const usageTimestamp =
                  sortMode === "session_recent"
                    ? item.session_last_used_at
                    : item.profile_last_used_at;
                const geoSummary = buildGeoSummary(locale, item);
                const metaParts = [item.source_label, item.primary_ip, geoSummary].filter(Boolean);
                return (
                  <button
                    key={item.node_id}
                    type="button"
                    onClick={() => setSelectedNodeId(item.node_id)}
                    className={cn(
                      "w-full rounded-2xl border px-4 py-3 text-left transition-colors",
                      selected
                        ? "border-primary bg-primary/5 shadow-sm"
                        : "border-border/70 bg-background hover:bg-muted/30",
                    )}
                  >
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div className="min-w-0 space-y-1">
                        <div className="flex flex-wrap items-center gap-2">
                          <div className="font-medium text-foreground">{item.proxy_name}</div>
                          {current ? <Badge variant="outline">{t("Current")}</Badge> : null}
                          {selected && !current ? <Badge>{t("Selected")}</Badge> : null}
                        </div>
                        {metaParts.length > 0 ? (
                          <div className="text-xs text-muted-foreground">
                            {metaParts.join(" · ")}
                          </div>
                        ) : null}
                      </div>
                      <div className="shrink-0 text-right text-xs text-muted-foreground">
                        <div>
                          {sortMode === "session_recent"
                            ? t("Current session last used")
                            : t("Current profile last used")}
                        </div>
                        <div className="mt-1 font-medium text-foreground">
                          {formatTimestamp(locale, t, usageTimestamp)}
                        </div>
                      </div>
                    </div>
                    <div className="mt-3 flex flex-wrap gap-2 text-xs text-muted-foreground">
                      <span>
                        {item.last_probe_ok === false
                          ? t("Probe failed")
                          : item.last_probe_ok === true
                            ? formatLatency(locale, t, item.median_latency_ms)
                            : t("No probe data yet")}
                      </span>
                      {item.profile_last_used_at ? (
                        <span>
                          {t("Profile last used {time}", {
                            time: formatTimestamp(locale, t, item.profile_last_used_at),
                          })}
                        </span>
                      ) : null}
                    </div>
                  </button>
                );
              })
            : null}
        </div>

        {error ? (
          <div className="rounded-2xl border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive">
            {error}
          </div>
        ) : null}

        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)}>
            {t("Cancel")}
          </Button>
          <Button
            onClick={() => {
              if (!session || !selectedNodeId) {
                return;
              }
              void onSubmit(session.session_id, { node_id: selectedNodeId });
            }}
            disabled={submitDisabled}
          >
            {isPending ? (
              <>
                <LoaderCircleIcon className="mr-2 size-4 animate-spin" />
                {t("Switching proxy…")}
              </>
            ) : (
              <>
                <PencilLineIcon className="mr-2 size-4" />
                {t("Use selected node")}
              </>
            )}
          </Button>
        </DialogFooter>

        {selectedItem && selectedItem.node_id !== session?.node_id ? (
          <div className="text-xs text-muted-foreground">
            {t("Switch to {proxyName} via {primaryIp}", {
              proxyName: selectedItem.proxy_name,
              primaryIp: selectedItem.primary_ip ?? t("No resolved IPs"),
            })}
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
