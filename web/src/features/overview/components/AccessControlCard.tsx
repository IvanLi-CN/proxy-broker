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
  ApiKeyProfileScope,
  ApiKeySummary,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
  CurrentUserState,
  SessionOptionItem,
} from "@/lib/types";

interface AccessControlCardProps {
  currentUser: CurrentUserState;
  currentProfileId: string;
  availableProfiles: string[];
  apiKeys: ApiKeySummary[];
  latestCreatedKey?: CreateApiKeyResponse | null;
  apiKeysLoading?: boolean;
  apiKeysError?: string | null;
  creatingApiKey?: boolean;
  revokingKeyId?: string | null;
  onCreateApiKey: (payload: CreateApiKeyRequest) => Promise<void> | void;
  onRevokeApiKey: (keyId: string) => Promise<void> | void;
}

function profileScopeSummary(scope: ApiKeyProfileScope, t: Translator) {
  if (scope.kind === "all_profiles") {
    return t("all profiles");
  }

  const profileIds = scope.profile_ids ?? [];
  if (profileIds.length === 1) {
    return t("profile {profileId}", { profileId: profileIds[0] });
  }
  if (profileIds.length === 2) {
    return profileIds.join(" / ");
  }
  return t("{count} selected profiles", { count: profileIds.length });
}

function profileScopeDetail(scope: ApiKeyProfileScope, t: Translator) {
  if (scope.kind === "all_profiles") {
    return t("all profiles");
  }

  const profileIds = scope.profile_ids ?? [];
  if (profileIds.length <= 3) {
    return profileIds.join(" / ");
  }
  return t("{count} selected profiles", { count: profileIds.length });
}

export function AccessControlCard({
  currentUser,
  currentProfileId,
  availableProfiles,
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
  const [allowAllProfiles, setAllowAllProfiles] = useState(false);
  const [selectedProfiles, setSelectedProfiles] = useState<string[]>(
    currentProfileId ? [currentProfileId] : [],
  );
  const canManageKeys =
    currentUser.status === "resolved" &&
    (currentUser.identity.is_admin || currentUser.identity.principal_type === "development");
  const normalizedProfiles = useMemo(
    () =>
      Array.from(new Set(availableProfiles.filter(Boolean))).sort((left, right) =>
        left.localeCompare(right),
      ),
    [availableProfiles],
  );

  useEffect(() => {
    setKeyName("");
    setAllowAllProfiles(false);
    setSelectedProfiles(currentProfileId ? [currentProfileId] : []);
  }, [currentProfileId]);

  const handleCreate = async () => {
    const nextName = keyName.trim();
    if (!nextName || !canManageKeys || (!allowAllProfiles && selectedProfiles.length === 0)) {
      return;
    }

    await onCreateApiKey({
      name: nextName,
      profile_scope: allowAllProfiles
        ? { kind: "all_profiles" }
        : { kind: "selected_profiles", profile_ids: selectedProfiles },
    });
    setKeyName("");
    setAllowAllProfiles(false);
    setSelectedProfiles(currentProfileId ? [currentProfileId] : []);
  };

  const searchProfiles = async (query: string): Promise<SessionOptionItem[]> => {
    const normalizedQuery = query.trim().toLowerCase();
    return normalizedProfiles
      .filter((profileId) => !normalizedQuery || profileId.toLowerCase().includes(normalizedQuery))
      .map((profileId) => ({
        value: profileId,
        label: profileId,
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
                  value: profileScopeSummary(latestCreatedKey.api_key.profile_scope, t),
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
                id="api-key-all-profiles"
                checked={allowAllProfiles}
                disabled={!canManageKeys}
                onCheckedChange={(checked) => setAllowAllProfiles(checked === true)}
              />
              <div className="space-y-1">
                <Label htmlFor="api-key-all-profiles">{t("Allow all profiles")}</Label>
                <p className="text-xs leading-5 text-muted-foreground">
                  {t("All future profiles remain available to this key until it is revoked.")}
                </p>
              </div>
            </div>
            {!allowAllProfiles ? (
              <SearchableMultiSelect
                id="api-key-profile-scope"
                label={t("Available profiles")}
                helper={t("The new key may access only the selected profiles.")}
                placeholder={t("Select one or more profiles")}
                searchPlaceholder={t("Search profiles")}
                emptyText={t("No matching profiles")}
                values={selectedProfiles}
                disabled={!canManageKeys}
                searchKey={`profiles:${normalizedProfiles.join(",")}`}
                onChange={setSelectedProfiles}
                onSearch={searchProfiles}
              />
            ) : null}
            <Button
              className="w-full sm:w-auto"
              onClick={() => void handleCreate()}
              disabled={
                creatingApiKey ||
                !keyName.trim() ||
                !canManageKeys ||
                (!allowAllProfiles && selectedProfiles.length === 0)
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
                        {profileScopeSummary(apiKey.profile_scope, t)}
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
                      {t("Scope {value}", { value: profileScopeDetail(apiKey.profile_scope, t) })}
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
