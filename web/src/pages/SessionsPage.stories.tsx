import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { batchFixture, sessionFixture, sessionsFixture } from "@/mocks/fixtures";
import { SessionsPage } from "@/pages/SessionsPage";

const meta = {
  title: "Pages/SessionsPage",
  component: SessionsPage,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Session route that combines single-open, batch-open, and a live listener deck inside the control-room layout.",
      },
    },
  },
  args: {
    sessions: sessionsFixture.sessions,
    sessionsLoading: false,
    initialized: true,
    initializationLoading: false,
    profileId: "default",
    openFormValues: {
      specifiedIp: "",
      desiredPort: "10080",
      countryCodes: "JP",
      cities: "",
      selectorSpecifiedIps: "",
      blacklistIps: "",
      limit: "1",
      sortMode: "lru",
    },
    onOpenFormValuesChange: fn(),
    batchFormValues: {
      requests: [
        {
          specifiedIp: "",
          desiredPort: "10080",
          countryCodes: "JP",
          cities: "Tokyo",
          selectorSpecifiedIps: "",
          blacklistIps: "",
          limit: "1",
          sortMode: "lru",
        },
      ],
    },
    onBatchFormValuesChange: fn(),
    openError: null,
    batchError: null,
    openResponse: sessionFixture,
    batchResponse: batchFixture,
    opening: false,
    batchOpening: false,
    closingSessionId: null,
    onOpenSession: fn(),
    onOpenBatch: fn(),
    onCloseSession: fn(),
  },
} satisfies Meta<typeof SessionsPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

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

export const UninitializedProject: Story = {
  args: {
    initialized: false,
    sessions: [],
  },
};
