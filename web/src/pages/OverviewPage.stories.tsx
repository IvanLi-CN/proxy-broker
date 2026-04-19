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
    currentProfileId: "default",
    availableProfiles: ["default", "edge-jp", "lab-us"],
    health: healthFixture,
    activeSessions: sessionsFixture.sessions.length,
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
        key_id: "key-Z4x6Cv8Bn1Mq3Rt5",
        profile_id: "default",
        name: "deploy-bot",
        prefix: "pbk_key-Z4x6Cv8Bn1",
        created_by: "admin@example.com",
        owner_subject: "admin@example.com",
        profile_scope: {
          kind: "selected_profiles",
          profile_ids: ["default", "edge-jp"],
        },
        created_at: 1_742_447_800,
        last_used_at: 1_742_448_400,
        revoked_at: null,
      },
    ],
    latestCreatedApiKey: null,
    apiKeysLoading: false,
    apiKeysError: null,
    creatingApiKey: false,
    revokingApiKeyId: null,
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
    await expect(canvas.getByRole("heading", { name: /overview/i })).toBeVisible();
    await expect(canvas.getByRole("button", { name: /refresh metadata/i })).toBeVisible();
  },
};

export const ZhCN: Story = {
  globals: {
    locale: "zh-CN",
  },
};

export const ErrorState: Story = {
  args: {
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

export const AllProfilesKeyState: Story = {
  args: {
    currentUser: {
      status: "resolved",
      identity: {
        authenticated: true,
        principal_type: "development",
        subject: "dev@local",
        email: "dev@local",
        groups: ["proxy-broker-dev-admin"],
        is_admin: true,
      },
    },
    apiKeys: [
      {
        key_id: "key-R2p8Ls4Dw7Hy1Ku6",
        profile_id: null,
        name: "fleet-bot",
        prefix: "pbk_key-R2p8Ls4Dw7",
        created_by: "dev@local",
        owner_subject: "dev@local",
        profile_scope: {
          kind: "all_profiles",
        },
        created_at: 1_742_449_800,
        last_used_at: 1_742_450_100,
        revoked_at: null,
      },
    ],
  },
};

export const AnonymousState: Story = {
  args: {
    currentUser: {
      status: "anonymous",
    },
    apiKeys: [],
    activeSessions: 0,
    refreshResponse: null,
  },
};
