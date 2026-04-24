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
          "Polling table of current sessions with inline proxy switching, selected IP geography, and empty/loading fallbacks.",
      },
    },
  },
  args: {
    sessions: sessionsFixture.sessions,
    isLoading: false,
    closingSessionId: null,
    switchingSessionId: null,
    onEditSession: fn(),
    onCloseSession: fn(),
  },
} satisfies Meta<typeof SessionsTable>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};

export const Loading: Story = {
  args: {
    sessions: [],
    isLoading: true,
    onEditSession: fn(),
  },
};

export const Empty: Story = {
  args: {
    sessions: [],
    isLoading: false,
    onEditSession: fn(),
  },
};

export const Switching: Story = {
  args: {
    switchingSessionId: sessionsFixture.sessions[0]?.session_id,
  },
};
