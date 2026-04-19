import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { AccessControlCard } from "@/features/overview/components/AccessControlCard";

const meta = {
  title: "Features/Overview/AccessControlCard",
  component: AccessControlCard,
  tags: ["autodocs"],
  args: {
    currentProfileId: "edge-jp",
    availableProfiles: ["default", "edge-jp", "lab-us"],
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
        profile_id: "edge-jp",
        name: "deploy-bot",
        prefix: "pbk_key-1_123456789",
        created_by: "admin@example.com",
        owner_subject: "admin@example.com",
        profile_scope: {
          kind: "selected_profiles",
          profile_ids: ["edge-jp"],
        },
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
          "Shows the resolved operator identity and the owner-scoped machine API keys issued by administrators.",
      },
    },
  },
} satisfies Meta<typeof AccessControlCard>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const MultiSelected: Story = {
  args: {
    onCreateApiKey: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("combobox", { name: /available profiles/i }));

    const overlay = within(document.body);
    await waitFor(() => expect(overlay.getByText("lab-us")).toBeVisible());
    await userEvent.click(overlay.getByText("lab-us"));
    await userEvent.click(canvas.getByRole("combobox", { name: /available profiles/i }));
    await userEvent.type(canvas.getByLabelText(/api key name/i), "multi-bot");
    await userEvent.click(canvas.getByRole("button", { name: /create key/i }));

    await waitFor(() => {
      expect(args.onCreateApiKey).toHaveBeenCalledWith({
        name: "multi-bot",
        profile_scope: {
          kind: "selected_profiles",
          profile_ids: ["edge-jp", "lab-us"],
        },
      });
    });
  },
};

export const AllProfiles: Story = {
  args: {
    apiKeys: [
      {
        key_id: "key-3",
        profile_id: null,
        name: "fleet-bot",
        prefix: "pbk_key-3_scopeall",
        created_by: "admin@example.com",
        owner_subject: "admin@example.com",
        profile_scope: {
          kind: "all_profiles",
        },
        created_at: 1_742_450_000,
        last_used_at: 1_742_450_360,
        revoked_at: null,
      },
    ],
    onCreateApiKey: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.type(canvas.getByLabelText(/api key name/i), "fleet-bot");
    await userEvent.click(canvas.getByLabelText(/allow all profiles/i));
    await userEvent.click(canvas.getByRole("button", { name: /create key/i }));

    await waitFor(() => {
      expect(args.onCreateApiKey).toHaveBeenCalledWith({
        name: "fleet-bot",
        profile_scope: {
          kind: "all_profiles",
        },
      });
    });
  },
};

export const WithFreshSecret: Story = {
  args: {
    latestCreatedKey: {
      api_key: {
        key_id: "key-2",
        profile_id: "edge-jp",
        name: "ci-runner",
        prefix: "pbk_key-2_abcdefghi",
        created_by: "admin@example.com",
        owner_subject: "admin@example.com",
        profile_scope: {
          kind: "selected_profiles",
          profile_ids: ["edge-jp", "lab-us"],
        },
        created_at: 1_742_449_000,
        last_used_at: null,
        revoked_at: null,
      },
      secret: "pbk_key-2_abcd1234efgh5678",
    },
  },
};

export const AnonymousOperator: Story = {
  args: {
    currentProfileId: "edge-jp",
    availableProfiles: ["default", "edge-jp", "lab-us"],
    currentUser: {
      status: "anonymous",
    },
    apiKeys: [],
  },
};

export const DevelopmentOperator: Story = {
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
  },
};
