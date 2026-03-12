import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { SessionsTable } from "@/features/sessions/components/SessionsTable";
import { sessionsFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Sessions/SessionsTable",
  component: SessionsTable,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Polling table of currently active sessions with close actions and empty/loading fallbacks.",
      },
    },
  },
  args: {
    sessions: sessionsFixture.sessions,
    isLoading: false,
    closingSessionId: null,
    onCloseSession: fn(),
  },
} satisfies Meta<typeof SessionsTable>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Loading: Story = {
  args: {
    sessions: [],
    isLoading: true,
  },
};

export const Empty: Story = {
  args: {
    sessions: [],
    isLoading: false,
  },
};
