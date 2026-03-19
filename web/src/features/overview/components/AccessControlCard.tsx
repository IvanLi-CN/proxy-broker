import { ShieldCheckIcon, ShieldEllipsisIcon, Trash2Icon } from "lucide-react";
import { useState } from "react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { formatTimestamp } from "@/lib/format";
import type { ApiKeySummary, AuthMeResponse, CreateApiKeyResponse } from "@/lib/types";

interface AccessControlCardProps {
  identity: AuthMeResponse | null;
  apiKeys: ApiKeySummary[];
  latestCreatedKey?: CreateApiKeyResponse | null;
  apiKeysLoading?: boolean;
  apiKeysError?: string | null;
  creatingApiKey?: boolean;
  revokingKeyId?: string | null;
  onCreateApiKey: (name: string) => Promise<void> | void;
  onRevokeApiKey: (keyId: string) => Promise<void> | void;
}

export function AccessControlCard({
  identity,
  apiKeys,
  latestCreatedKey = null,
  apiKeysLoading = false,
  apiKeysError = null,
  creatingApiKey = false,
  revokingKeyId = null,
  onCreateApiKey,
  onRevokeApiKey,
}: AccessControlCardProps) {
  const [keyName, setKeyName] = useState("");

  const handleCreate = async () => {
    const nextName = keyName.trim();
    if (!nextName) {
      return;
    }
    await onCreateApiKey(nextName);
    setKeyName("");
  };

  return (
    <Card className="border-border/70 bg-card/96 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
      <CardHeader className="space-y-3 border-b border-border/70 pb-5">
        <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
          Access control
        </div>
        <CardTitle className="text-xl tracking-tight">Identity and project keys</CardTitle>
        <CardDescription className="text-sm leading-6 text-muted-foreground">
          Forward Auth only tells the backend who the operator is. Admin checks and profile-scoped
          machine keys are enforced here.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-5 pt-6">
        <div className="rounded-2xl border border-border/70 bg-muted/15 p-4">
          <div className="flex items-center gap-2 text-sm font-medium text-foreground">
            {identity?.is_admin ? (
              <ShieldCheckIcon className="size-4 text-emerald-500" />
            ) : (
              <ShieldEllipsisIcon className="size-4 text-amber-500" />
            )}
            Current operator
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <Badge variant="outline" className="rounded-full px-3 py-1 font-mono text-[11px]">
              {identity?.principal_type ?? "unknown"}
            </Badge>
            <Badge variant="secondary" className="rounded-full px-3 py-1 font-mono text-[11px]">
              {identity?.subject ?? "unresolved"}
            </Badge>
            {identity?.is_admin ? (
              <Badge className="rounded-full bg-emerald-500/15 text-emerald-700 dark:text-emerald-300">
                admin
              </Badge>
            ) : null}
          </div>
          {identity?.email ? (
            <p className="mt-3 text-sm text-muted-foreground">Email: {identity.email}</p>
          ) : null}
          {identity?.groups.length ? (
            <p className="mt-2 text-sm text-muted-foreground">
              Groups: {identity.groups.join(" / ")}
            </p>
          ) : null}
        </div>

        {latestCreatedKey ? (
          <div className="space-y-3">
            <ActionResponsePanel
              title="New API key issued"
              description="Copy this secret now. The backend will only reveal it once."
              bullets={[
                `profile ${latestCreatedKey.api_key.profile_id}`,
                `prefix ${latestCreatedKey.api_key.prefix}`,
              ]}
            />
            <pre className="overflow-x-auto rounded-2xl border border-border/70 bg-background px-4 py-3 text-xs leading-6 text-foreground">
              {latestCreatedKey.secret}
            </pre>
          </div>
        ) : null}

        <div className="space-y-3">
          <div className="text-sm font-medium text-foreground">Create a profile key</div>
          <div className="flex gap-3">
            <Input
              aria-label="API key name"
              placeholder="deploy-bot"
              value={keyName}
              onChange={(event) => setKeyName(event.target.value)}
            />
            <Button
              onClick={() => void handleCreate()}
              disabled={creatingApiKey || !keyName.trim()}
            >
              Create key
            </Button>
          </div>
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between gap-3">
            <div className="text-sm font-medium text-foreground">Issued keys</div>
            <Badge variant="outline" className="rounded-full px-3 py-1 font-mono text-[11px]">
              {apiKeys.length} total
            </Badge>
          </div>
          {apiKeysError ? (
            <ActionResponsePanel
              title="Key inventory unavailable"
              tone="error"
              description={apiKeysError}
            />
          ) : null}
          {apiKeysLoading ? (
            <div className="rounded-2xl border border-dashed border-border/70 px-4 py-6 text-sm text-muted-foreground">
              Loading issued keys...
            </div>
          ) : null}
          {!apiKeysLoading && apiKeys.length === 0 ? (
            <div className="rounded-2xl border border-dashed border-border/70 px-4 py-6 text-sm text-muted-foreground">
              No machine keys have been issued for this profile yet.
            </div>
          ) : null}
          {!apiKeysLoading && apiKeys.length > 0 ? (
            <div className="space-y-3">
              {apiKeys.map((apiKey) => (
                <div
                  key={apiKey.key_id}
                  className="flex flex-col gap-3 rounded-2xl border border-border/70 bg-background/80 p-4 shadow-sm"
                >
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <div>
                      <div className="text-sm font-semibold text-foreground">{apiKey.name}</div>
                      <div className="mt-1 font-mono text-xs text-muted-foreground">
                        {apiKey.prefix}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      {apiKey.revoked_at ? (
                        <Badge variant="secondary" className="rounded-full">
                          revoked
                        </Badge>
                      ) : (
                        <Badge className="rounded-full bg-sky-500/15 text-sky-700 dark:text-sky-300">
                          active
                        </Badge>
                      )}
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={Boolean(apiKey.revoked_at) || revokingKeyId === apiKey.key_id}
                        onClick={() => void onRevokeApiKey(apiKey.key_id)}
                      >
                        <Trash2Icon className="size-4" />
                        Revoke
                      </Button>
                    </div>
                  </div>
                  <div className="grid gap-1 text-xs leading-5 text-muted-foreground">
                    <div>Created by {apiKey.created_by}</div>
                    <div>Created {formatTimestamp(apiKey.created_at)}</div>
                    <div>Last used {formatTimestamp(apiKey.last_used_at)}</div>
                    {apiKey.revoked_at ? (
                      <div>Revoked {formatTimestamp(apiKey.revoked_at)}</div>
                    ) : null}
                  </div>
                </div>
              ))}
            </div>
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}
