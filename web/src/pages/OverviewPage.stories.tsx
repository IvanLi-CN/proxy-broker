import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import {
  healthFixture,
  refreshFixture,
  sessionsFixture,
  subscriptionFixture,
} from "@/mocks/fixtures";
import { OverviewPage } from "@/pages/OverviewPage";

const meta = {
  title: "Pages/OverviewPage",
  component: OverviewPage,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Overview route composition for health, subscription loading, refresh actions, and runway guidance inside the operator control room.",
      },
    },
  },
  args: {
    health: healthFixture,
    activeSessions: sessionsFixture.sessions.length,
    loadResponse: subscriptionFixture,
    loadError: null,
    refreshResponse: refreshFixture,
    refreshError: null,
    loadingSubscription: false,
    refreshing: false,
    identity: {
      authenticated: true,
      principal_type: "human",
      subject: "admin@example.com",
      email: "admin@example.com",
      groups: ["admins", "ops"],
      is_admin: true,
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
    apiKeysLoading: false,
    apiKeysError: null,
    creatingApiKey: false,
    revokingApiKeyId: null,
    onLoadSubscription: fn(),
    onRefresh: fn(),
    onCreateApiKey: fn(),
    onRevokeApiKey: fn(),
  },
} satisfies Meta<typeof OverviewPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ErrorState: Story = {
  args: {
    loadResponse: null,
    loadError: "subscription_invalid: malformed upstream payload",
    refreshResponse: null,
    refreshError: "mihomo_unavailable: controller not reachable",
  },
};

export const QuietState: Story = {
  args: {
    activeSessions: 0,
    loadResponse: null,
    refreshResponse: null,
  },
};
