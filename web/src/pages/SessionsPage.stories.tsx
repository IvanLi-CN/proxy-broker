import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { AppShell } from "@/components/AppShell";
import { batchFixture, sessionFixture, sessionsFixture } from "@/mocks/fixtures";
import { SessionsPage } from "@/pages/SessionsPage";

const meta = {
  title: "Pages/SessionsPage",
  component: SessionsPage,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    initialEntries: ["/sessions"],
    docs: {
      description: {
        component:
          "Session route inside the real app shell, opening directly on the single/batch forms and live listener deck without a route hero.",
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
      healthStatus="ok"
      currentUser={{
        status: "resolved",
        identity: {
          authenticated: true,
          principal_type: "human",
          subject: "admin@example.com",
          email: "admin@example.com",
          groups: ["admins", "ops"],
          is_admin: true,
        },
      }}
      onProfileIdChange={() => undefined}
      onCreateProfile={async (value: string) => value}
      onRetryProfiles={() => undefined}
    >
      <SessionsPage {...args} />
    </AppShell>
  ),
  args: {
    sessions: sessionsFixture.sessions,
    sessionsLoading: false,
    openError: null,
    batchError: null,
    openResponse: sessionFixture,
    batchResponse: batchFixture,
    opening: false,
    batchOpening: false,
    suggestedPort: 10080,
    closingSessionId: null,
    onOpenSession: fn(),
    onOpenBatch: fn(),
    searchSessionOptions: fn(async () => []),
    onCloseSession: fn(),
  },
} satisfies Meta<typeof SessionsPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ZhCN: Story = {
  globals: {
    locale: "zh-CN",
  },
};

export const EmptyState: Story = {
  args: {
    sessions: [],
    openResponse: null,
    batchResponse: null,
  },
};

export const ClosingState: Story = {
  args: {
    closingSessionId: sessionsFixture.sessions[0]?.session_id,
  },
};
