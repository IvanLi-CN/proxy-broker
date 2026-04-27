import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

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
  async play({ canvasElement }) {
    const dialog = within(canvasElement.ownerDocument.body);
    await expect((await dialog.findAllByText(/JP-Tokyo-Entry/i)).length).toBeGreaterThan(0);
    const latencyLabels = await dialog.findAllByText(/88 ms/i);
    await expect(latencyLabels.length).toBeGreaterThan(0);
    await expect((await dialog.findAllByText(/Current session last used/i)).length).toBeGreaterThan(
      0,
    );
    const firstLatencyLabel = latencyLabels[0];
    if (!firstLatencyLabel) {
      throw new Error("Expected an 88 ms latency label");
    }
    await userEvent.hover(firstLatencyLabel);
    await expect((await dialog.findAllByText(/140 ms/i)).length).toBeGreaterThan(0);
  },
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
