import { AccessControlCard } from "@/features/overview/components/AccessControlCard";
import { HealthSummaryCard } from "@/features/overview/components/HealthSummaryCard";
import { RefreshCard } from "@/features/overview/components/RefreshCard";
import { SubscriptionFormCard } from "@/features/overview/components/SubscriptionFormCard";
import { useI18n } from "@/i18n";
import type {
  ApiKeySummary,
  CreateApiKeyResponse,
  CurrentUserState,
  HealthResponse,
  LoadSubscriptionRequest,
  LoadSubscriptionResponse,
  RefreshRequest,
  RefreshResponse,
} from "@/lib/types";

interface OverviewPageProps {
  health: HealthResponse;
  activeSessions: number;
  loadResponse?: LoadSubscriptionResponse | null;
  loadError?: string | null;
  refreshResponse?: RefreshResponse | null;
  refreshError?: string | null;
  loadingSubscription: boolean;
  refreshing: boolean;
  currentUser: CurrentUserState;
  apiKeys?: ApiKeySummary[];
  latestCreatedApiKey?: CreateApiKeyResponse | null;
  apiKeysLoading?: boolean;
  apiKeysError?: string | null;
  creatingApiKey?: boolean;
  revokingApiKeyId?: string | null;
  onLoadSubscription: (payload: LoadSubscriptionRequest) => void | Promise<void>;
  onRefresh: (payload: RefreshRequest) => void | Promise<void>;
  onCreateApiKey: (name: string) => void | Promise<void>;
  onRevokeApiKey: (keyId: string) => void | Promise<void>;
}

export function OverviewPage({
  health,
  activeSessions,
  loadResponse,
  loadError,
  refreshResponse,
  refreshError,
  loadingSubscription,
  refreshing,
  currentUser,
  apiKeys = [],
  latestCreatedApiKey = null,
  apiKeysLoading = false,
  apiKeysError = null,
  creatingApiKey = false,
  revokingApiKeyId = null,
  onLoadSubscription,
  onRefresh,
  onCreateApiKey,
  onRevokeApiKey,
}: OverviewPageProps) {
  const { t } = useI18n();
  const hasWarnings = Boolean(loadResponse?.warnings.length);

  return (
    <div className="space-y-8">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">{t("Overview")}</h1>
      </header>

      <HealthSummaryCard
        status={health.status}
        activeSessions={activeSessions}
        hasWarnings={hasWarnings}
        loadedProxies={loadResponse?.loaded_proxies ?? null}
        refreshedIps={refreshResponse?.probed_ips ?? null}
      />

      <section className="grid gap-6 xl:grid-cols-[minmax(0,1.2fr)_360px]">
        <div className="space-y-6">
          <SubscriptionFormCard
            error={loadError}
            isPending={loadingSubscription}
            onSubmit={onLoadSubscription}
            response={loadResponse}
          />
          <RefreshCard
            error={refreshError}
            isPending={refreshing}
            onSubmit={onRefresh}
            response={refreshResponse}
          />
        </div>

        <div className="space-y-6">
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
