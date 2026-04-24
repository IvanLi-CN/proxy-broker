import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { SessionNodeSelectDialog } from "@/features/sessions/components/SessionNodeSelectDialog";
import { sessionNodeOptionsFixture, sessionsFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Sessions/SessionNodeSelectDialog",
  component: SessionNodeSelectDialog,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Dialog for switching the current session to another node, with keyword filtering and recency-based sorting.",
      },
    },
  },
  args: {
    open: true,
    session: sessionsFixture.sessions[0] ?? null,
    isPending: false,
    error: null,
    onOpenChange: fn(),
    onSearch: fn(async () => sessionNodeOptionsFixture.items),
    onSubmit: fn(),
  },
} satisfies Meta<typeof SessionNodeSelectDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};

export const Switching: Story = {
  args: {
    isPending: true,
  },
};

export const ZhCN: Story = {
  args: {},
  globals: {
    locale: "zh-CN",
  },
};
