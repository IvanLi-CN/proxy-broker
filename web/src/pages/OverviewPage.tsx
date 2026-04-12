import { AccessControlCard } from "@/features/overview/components/AccessControlCard";
import { HealthSummaryCard } from "@/features/overview/components/HealthSummaryCard";
import { RefreshCard } from "@/features/overview/components/RefreshCard";
import { ProfileProxyPolicyCard } from "@/features/proxies/components/ProfileProxyPolicyCard";
import { ProxyLoadCard } from "@/features/proxies/components/ProxyLoadCard";
import { useI18n } from "@/i18n";
import type {
  ApiKeySummary,
  CreateApiKeyResponse,
  CurrentUserState,
  HealthResponse,
  LoadSubscriptionRequest,
  LoadSubscriptionResponse,
  ProfileProxySettings,
  RefreshRequest,
  RefreshResponse,
} from "@/lib/types";

interface OverviewPageProps {
  profileId: string;
  health: HealthResponse;
  activeSessions: number;
  profileLoadResponse?: LoadSubscriptionResponse | null;
  profileLoadError?: string | null;
  loadingProfile?: boolean;
  refreshResponse?: RefreshResponse | null;
  refreshError?: string | null;
  refreshing: boolean;
  currentUser: CurrentUserState;
  proxySettings?: ProfileProxySettings | null;
  proxySettingsLoading?: boolean;
  proxySettingsError?: string | null;
  updatingSettings?: boolean;
  showProxyPolicy?: boolean;
  apiKeys?: ApiKeySummary[];
  latestCreatedApiKey?: CreateApiKeyResponse | null;
  apiKeysLoading?: boolean;
  apiKeysError?: string | null;
  creatingApiKey?: boolean;
  revokingApiKeyId?: string | null;
  onLoadProfile: (payload: LoadSubscriptionRequest) => void | Promise<void>;
  onToggleUseGlobalProxies: (nextValue: boolean) => void | Promise<void>;
  onRefresh: (payload: RefreshRequest) => void | Promise<void>;
  onCreateApiKey: (name: string) => void | Promise<void>;
  onRevokeApiKey: (keyId: string) => void | Promise<void>;
}

export function OverviewPage({
  profileId,
  health,
  activeSessions,
  profileLoadResponse,
  profileLoadError,
  loadingProfile = false,
  refreshResponse,
  refreshError,
  refreshing,
  currentUser,
  proxySettings,
  proxySettingsLoading = false,
  proxySettingsError = null,
  updatingSettings = false,
  showProxyPolicy = true,
  apiKeys = [],
  latestCreatedApiKey = null,
  apiKeysLoading = false,
  apiKeysError = null,
  creatingApiKey = false,
  revokingApiKeyId = null,
  onLoadProfile,
  onToggleUseGlobalProxies,
  onRefresh,
  onCreateApiKey,
  onRevokeApiKey,
}: OverviewPageProps) {
  const { t } = useI18n();
  const useGlobalProxies = proxySettings?.use_global_proxies ?? true;

  return (
    <div className="space-y-8">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">{t("Overview")}</h1>
      </header>

      <HealthSummaryCard
        status={health.status}
        activeSessions={activeSessions}
        hasWarnings={false}
        loadedProxies={null}
        refreshedIps={refreshResponse?.probed_ips ?? null}
      />

      <section className="grid gap-6 xl:grid-cols-[minmax(0,1.2fr)_360px]">
        <div className="space-y-6">
          <ProxyLoadCard
            defaultValue="https://example.com/profile-subscription.yaml"
            description={t(
              "Import nodes for the current profile only. These nodes stay local unless you later reassign them from the global inventory.",
            )}
            error={profileLoadError}
            eyebrow={t("Current profile")}
            onSubmit={onLoadProfile}
            pending={loadingProfile}
            response={profileLoadResponse}
            scopeChip={t("allocation defaults to {profileId}", { profileId })}
            submitLabel={t("Import profile pool")}
            successDescription={t(
              "Imported {proxyCount} proxies across {ipCount} distinct IPs into profile {profileId}.",
              {
                proxyCount: profileLoadResponse?.loaded_proxies ?? 0,
                ipCount: profileLoadResponse?.distinct_ips ?? 0,
                profileId,
              },
            )}
            successTitle={t("Profile pool updated")}
            title={t("Import local pool for {profileId}", { profileId })}
          />
          <RefreshCard
            error={refreshError}
            isPending={refreshing}
            onSubmit={onRefresh}
            response={refreshResponse}
          />
        </div>

        <div className="space-y-6">
          {showProxyPolicy ? (
            <ProfileProxyPolicyCard
              profileId={profileId}
              proxySettingsError={proxySettingsError}
              proxySettingsLoading={proxySettingsLoading}
              updatingSettings={updatingSettings}
              useGlobalProxies={useGlobalProxies}
              onToggleUseGlobalProxies={onToggleUseGlobalProxies}
            />
          ) : null}
          <AccessControlCard
            currentUser={currentUser}
            apiKeys={apiKeys}
            latestCreatedKey={latestCreatedApiKey}
            apiKeysLoading={apiKeysLoading}
            apiKeysError={apiKeysError}
            creatingApiKey={creatingApiKey}
            revokingKeyId={revokingApiKeyId}
            onCreateApiKey={onCreateApiKey}
            onRevokeApiKey={onRevokeApiKey}
          />
        </div>
      </section>
    </div>
  );
}
