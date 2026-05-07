import { Trash2Icon } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { CurrentUserSummary } from "@/components/CurrentUserSummary";
import { SearchableMultiSelect } from "@/components/SearchableMultiSelect";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { type Translator, useI18n } from "@/i18n";
import { formatTimestamp } from "@/lib/format";
import type {
  ApiKeyProjectScope,
  ApiKeySummary,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
  CurrentUserState,
  SessionOptionItem,
} from "@/lib/types";

interface AccessControlCardProps {
  currentUser: CurrentUserState;
  currentProjectId: string;
  availableProjects: string[];
  apiKeys: ApiKeySummary[];
  latestCreatedKey?: CreateApiKeyResponse | null;
  apiKeysLoading?: boolean;
  apiKeysError?: string | null;
  creatingApiKey?: boolean;
  revokingKeyId?: string | null;
  onCreateApiKey: (payload: CreateApiKeyRequest) => Promise<void> | void;
  onRevokeApiKey: (keyId: string) => Promise<void> | void;
}

function projectScopeSummary(scope: ApiKeyProjectScope, t: Translator) {
  if (scope.kind === "all_projects") {
    return t("all projects");
  }

  const projectIds = scope.project_ids ?? [];
  if (projectIds.length === 1) {
    return t("project {projectId}", { projectId: projectIds[0] });
  }
  if (projectIds.length === 2) {
    return projectIds.join(" / ");
  }
  return t("{count} selected projects", { count: projectIds.length });
}

function projectScopeDetail(scope: ApiKeyProjectScope, t: Translator) {
  if (scope.kind === "all_projects") {
    return t("all projects");
  }

  const projectIds = scope.project_ids ?? [];
  if (projectIds.length <= 3) {
    return projectIds.join(" / ");
  }
  return t("{count} selected projects", { count: projectIds.length });
}

export function AccessControlCard({
  currentUser,
  currentProjectId,
  availableProjects,
  apiKeys,
  latestCreatedKey = null,
  apiKeysLoading = false,
  apiKeysError = null,
  creatingApiKey = false,
  revokingKeyId = null,
  onCreateApiKey,
  onRevokeApiKey,
}: AccessControlCardProps) {
  const { locale, t } = useI18n();
  const [keyName, setKeyName] = useState("");
  const [allowAllProjects, setAllowAllProjects] = useState(false);
  const [selectedProjects, setSelectedProjects] = useState<string[]>(
    currentProjectId ? [currentProjectId] : [],
  );
  const canManageKeys =
    currentUser.status === "resolved" &&
    (currentUser.identity.is_admin || currentUser.identity.principal_type === "development");
  const normalizedProjects = useMemo(
    () =>
      Array.from(new Set(availableProjects.filter(Boolean))).sort((left, right) =>
        left.localeCompare(right),
      ),
    [availableProjects],
  );

  useEffect(() => {
    setKeyName("");
    setAllowAllProjects(false);
    setSelectedProjects(currentProjectId ? [currentProjectId] : []);
  }, [currentProjectId]);

  const handleCreate = async () => {
    const nextName = keyName.trim();
    if (!nextName || !canManageKeys || (!allowAllProjects && selectedProjects.length === 0)) {
      return;
    }

    await onCreateApiKey({
      name: nextName,
      project_scope: allowAllProjects
        ? { kind: "all_projects" }
        : { kind: "selected_projects", project_ids: selectedProjects },
    });
    setKeyName("");
    setAllowAllProjects(false);
    setSelectedProjects(currentProjectId ? [currentProjectId] : []);
  };

  const searchProjects = async (query: string): Promise<SessionOptionItem[]> => {
    const normalizedQuery = query.trim().toLowerCase();
    return normalizedProjects
      .filter((projectId) => !normalizedQuery || projectId.toLowerCase().includes(normalizedQuery))
      .map((projectId) => ({
        value: projectId,
        label: projectId,
      }));
  };

  return (
    <Card className="border-border/70 bg-card/96 shadow-[0_20px_60px_-42px_rgba(15,23,42,0.5)]">
      <CardHeader className="space-y-3 border-b border-border/70 pb-5">
        <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
          {t("Access control")}
        </div>
        <CardTitle className="text-xl tracking-tight">{t("Identity and project keys")}</CardTitle>
        <CardDescription className="text-sm leading-6 text-muted-foreground">
          {t(
            "Forward Auth only tells the backend who the operator is. Admin checks and owner-scoped machine keys are enforced here.",
          )}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-5 pt-6">
        <CurrentUserSummary currentUser={currentUser} />

        {latestCreatedKey ? (
          <div className="space-y-3">
            <ActionResponsePanel
              title={t("New API key issued")}
              description={t("Copy this secret now. The backend will only reveal it once.")}
              bullets={[
                t("owner {subject}", { subject: latestCreatedKey.api_key.owner_subject }),
                t("scope {value}", {
                  value: projectScopeSummary(latestCreatedKey.api_key.project_scope, t),
                }),
                t("prefix {prefix}", { prefix: latestCreatedKey.api_key.prefix }),
              ]}
            />
            <pre className="overflow-x-auto rounded-2xl border border-border/70 bg-background px-4 py-3 text-xs leading-6 text-foreground">
              {latestCreatedKey.secret}
            </pre>
          </div>
        ) : null}

        <div className="space-y-4">
          <div className="text-sm font-medium text-foreground">{t("Create an owner key")}</div>
          {!canManageKeys ? (
            <div className="rounded-2xl border border-dashed border-border/70 px-4 py-4 text-sm text-muted-foreground">
              {t("Machine keys can only be issued by an admin human or the development identity.")}
            </div>
          ) : null}
          <div className="space-y-3">
            <Input
              aria-label={t("API key name")}
              placeholder="deploy-bot"
              value={keyName}
              onChange={(event) => setKeyName(event.target.value)}
              disabled={!canManageKeys}
            />
            <div className="flex items-start gap-3 rounded-2xl border border-border/70 bg-background/60 px-4 py-3">
              <Checkbox
                id="api-key-all-projects"
                checked={allowAllProjects}
                disabled={!canManageKeys}
                onCheckedChange={(checked) => setAllowAllProjects(checked === true)}
              />
              <div className="space-y-1">
                <Label htmlFor="api-key-all-projects">{t("Allow all projects")}</Label>
                <p className="text-xs leading-5 text-muted-foreground">
                  {t("All future projects remain available to this key until it is revoked.")}
                </p>
              </div>
            </div>
            {!allowAllProjects ? (
              <SearchableMultiSelect
                id="api-key-project-scope"
                label={t("Available projects")}
                helper={t("The new key may access only the selected projects.")}
                placeholder={t("Select one or more projects")}
                searchPlaceholder={t("Search projects")}
                emptyText={t("No matching projects")}
                values={selectedProjects}
                disabled={!canManageKeys}
                searchKey={`projects:${normalizedProjects.join(",")}`}
                onChange={setSelectedProjects}
                onSearch={searchProjects}
              />
            ) : null}
            <Button
              className="w-full sm:w-auto"
              onClick={() => void handleCreate()}
              disabled={
                creatingApiKey ||
                !keyName.trim() ||
                !canManageKeys ||
                (!allowAllProjects && selectedProjects.length === 0)
              }
            >
              {t("Create key")}
            </Button>
          </div>
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between gap-3">
            <div className="text-sm font-medium text-foreground">{t("Issued keys")}</div>
            <Badge variant="outline" className="rounded-full px-3 py-1 font-mono text-[11px]">
              {t("{count} total", { count: apiKeys.length })}
            </Badge>
          </div>
          {apiKeysError ? (
            <ActionResponsePanel
              title={t("Key inventory unavailable")}
              tone="error"
              description={apiKeysError}
            />
          ) : null}
          {apiKeysLoading ? (
            <div className="rounded-2xl border border-dashed border-border/70 px-4 py-6 text-sm text-muted-foreground">
              {t("Loading issued keys...")}
            </div>
          ) : null}
          {!apiKeysLoading && apiKeys.length === 0 ? (
            <div className="rounded-2xl border border-dashed border-border/70 px-4 py-6 text-sm text-muted-foreground">
              {t("No machine keys have been issued for this owner yet.")}
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
                    <div className="flex flex-wrap items-center gap-2">
                      {apiKey.revoked_at ? (
                        <Badge variant="secondary" className="rounded-full">
                          {t("revoked")}
                        </Badge>
                      ) : (
                        <Badge className="rounded-full bg-sky-500/15 text-sky-700 dark:text-sky-300">
                          {t("active")}
                        </Badge>
                      )}
                      <Badge variant="outline" className="rounded-full">
                        {projectScopeSummary(apiKey.project_scope, t)}
                      </Badge>
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={
                          !canManageKeys ||
                          Boolean(apiKey.revoked_at) ||
                          revokingKeyId === apiKey.key_id
                        }
                        onClick={() => void onRevokeApiKey(apiKey.key_id)}
                      >
                        <Trash2Icon className="size-4" />
                        {t("Revoke")}
                      </Button>
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-x-6 gap-y-2 text-xs leading-5 text-muted-foreground max-[360px]:grid-cols-1">
                    <div className="min-w-0 break-words">
                      {t("Owner {subject}", { subject: apiKey.owner_subject })}
                    </div>
                    <div className="min-w-0 break-words">
                      {t("Scope {value}", { value: projectScopeDetail(apiKey.project_scope, t) })}
                    </div>
                    <div className="min-w-0 break-words">
                      {t("Created {value}", {
                        value: formatTimestamp(locale, t, apiKey.created_at),
                      })}
                    </div>
                    <div className="min-w-0 break-words">
                      {t("Last used {value}", {
                        value: formatTimestamp(locale, t, apiKey.last_used_at),
                      })}
                    </div>
                    {apiKey.revoked_at ? (
                      <div className="min-w-0 break-words sm:col-span-2">
                        {t("Revoked {value}", {
                          value: formatTimestamp(locale, t, apiKey.revoked_at),
                        })}
                      </div>
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
