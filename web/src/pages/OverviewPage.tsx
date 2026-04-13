import { AccessControlCard } from "@/features/overview/components/AccessControlCard";
import { HealthSummaryCard } from "@/features/overview/components/HealthSummaryCard";
import { RefreshCard } from "@/features/overview/components/RefreshCard";
import { useI18n } from "@/i18n";
import type {
  ApiKeySummary,
  CreateApiKeyResponse,
  CurrentUserState,
  HealthResponse,
  RefreshRequest,
  RefreshResponse,
} from "@/lib/types";

interface OverviewPageProps {
  health: HealthResponse;
  activeSessions: number;
  refreshResponse?: RefreshResponse | null;
  refreshError?: string | null;
  refreshing: boolean;
  currentUser: CurrentUserState;
  apiKeys?: ApiKeySummary[];
  latestCreatedApiKey?: CreateApiKeyResponse | null;
  apiKeysLoading?: boolean;
  apiKeysError?: string | null;
  creatingApiKey?: boolean;
  revokingApiKeyId?: string | null;
  onRefresh: (payload: RefreshRequest) => void | Promise<void>;
  onCreateApiKey: (name: string) => void | Promise<void>;
  onRevokeApiKey: (keyId: string) => void | Promise<void>;
}

export function OverviewPage({
  health,
  activeSessions,
  refreshResponse,
  refreshError,
  refreshing,
  currentUser,
  apiKeys = [],
  latestCreatedApiKey = null,
  apiKeysLoading = false,
  apiKeysError = null,
  creatingApiKey = false,
  revokingApiKeyId = null,
  onRefresh,
  onCreateApiKey,
  onRevokeApiKey,
}: OverviewPageProps) {
  const { t } = useI18n();

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
