import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { AccessControlCard } from "@/features/overview/components/AccessControlCard";

const meta = {
  title: "Features/Overview/AccessControlCard",
  component: AccessControlCard,
  tags: ["autodocs"],
  args: {
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
        profile_id: "edge-jp",
        name: "deploy-bot",
        prefix: "pbk_key-1_123456789",
        created_by: "admin@example.com",
        created_at: 1_742_447_800,
        last_used_at: 1_742_448_400,
        revoked_at: null,
      },
    ],
    latestCreatedKey: null,
    apiKeysLoading: false,
    apiKeysError: null,
    creatingApiKey: false,
    revokingKeyId: null,
    onCreateApiKey: fn(),
    onRevokeApiKey: fn(),
  },
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component:
          "Shows the resolved operator identity and the profile-scoped machine API keys issued by administrators.",
      },
    },
  },
} satisfies Meta<typeof AccessControlCard>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const WithFreshSecret: Story = {
  args: {
    latestCreatedKey: {
      api_key: {
        key_id: "key-2",
        profile_id: "edge-jp",
        name: "ci-runner",
        prefix: "pbk_key-2_abcdefghi",
        created_by: "admin@example.com",
        created_at: 1_742_449_000,
        last_used_at: null,
        revoked_at: null,
      },
      secret: "pbk_key-2_abcd1234efgh5678",
    },
  },
};
