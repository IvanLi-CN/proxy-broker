import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { AppShell } from "@/components/AppShell";
import { healthFixture, refreshFixture, sessionsFixture } from "@/mocks/fixtures";
import { OverviewPage } from "@/pages/OverviewPage";

const meta = {
  title: "Pages/OverviewPage",
  component: OverviewPage,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          "Full overview route preview inside the real app shell. This is the closest Storybook equivalent of the shipped operator page, with the route now opening directly on the health summary and primary operator cards.",
      },
    },
  },
  render: (args) => (
    <AppShell
      profileId="default"
      profiles={["default", "edge-jp", "lab-us"]}
      profilesLoading={false}
      profilesCreating={false}
      profilesError={null}
      healthStatus={args.health.status}
      currentUser={args.currentUser}
      onProfileIdChange={() => undefined}
      onCreateProfile={async (value: string) => value}
      onRetryProfiles={() => undefined}
    >
      <OverviewPage {...args} />
    </AppShell>
  ),
  args: {
    profileId: "default",
    health: healthFixture,
    activeSessions: sessionsFixture.sessions.length,
    profileLoadResponse: {
      loaded_proxies: 6,
      distinct_ips: 4,
      warnings: [],
    },
    profileLoadError: null,
    loadingProfile: false,
    refreshResponse: refreshFixture,
    refreshError: null,
    refreshing: false,
    currentUser: {
      status: "resolved",
      identity: {
        authenticated: true,
        principal_type: "human",
        subject: "admin@example.com",
        email: "admin@example.com",
        groups: ["admins", "ops"],
        is_admin: true,
      },
    },
    apiKeys: [
      {
        key_id: "key-1",
        profile_id: "default",
        name: "deploy-bot",
        prefix: "pbk_key-1_123456789",
        created_by: "admin@example.com",
        created_at: 1_742_447_800,
        last_used_at: 1_742_448_400,
        revoked_at: null,
      },
    ],
    latestCreatedApiKey: null,
    proxySettings: {
      profile_id: "default",
      use_global_proxies: true,
    },
    proxySettingsLoading: false,
    proxySettingsError: null,
    updatingSettings: false,
    showProxyPolicy: true,
    apiKeysLoading: false,
    apiKeysError: null,
    creatingApiKey: false,
    revokingApiKeyId: null,
    onLoadProfile: fn(),
    onToggleUseGlobalProxies: fn(),
    onRefresh: fn(),
    onCreateApiKey: fn(),
    onRevokeApiKey: fn(),
  },
} satisfies Meta<typeof OverviewPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await expect(
      canvas.getByRole("heading", { name: /import local pool for default/i }),
    ).toBeVisible();
    await expect(
      canvas.getByRole("heading", { name: /use global pool for default/i }),
    ).toBeVisible();
  },
};

export const ZhCN: Story = {
  globals: {
    locale: "zh-CN",
  },
};

export const ErrorState: Story = {
  args: {
    profileLoadResponse: null,
    profileLoadError: "subscription_fetch_failed: upstream not reachable",
    refreshResponse: null,
    refreshError: "mihomo_unavailable: controller not reachable",
  },
};

export const QuietState: Story = {
  args: {
    activeSessions: 0,
    refreshResponse: null,
  },
};

export const AnonymousState: Story = {
  args: {
    currentUser: {
      status: "anonymous",
    },
    apiKeys: [],
    activeSessions: 0,
    profileLoadResponse: null,
    refreshResponse: null,
    showProxyPolicy: false,
  },
};
