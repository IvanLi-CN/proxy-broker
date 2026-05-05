import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { SessionNodeSelectDialog } from "@/features/sessions/components/SessionNodeSelectDialog";
import { sessionIpNodeOptionsFixture, sessionsFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Sessions/SessionNodeSelectDialog",
  component: SessionNodeSelectDialog,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Dialog for switching the current session to an IP and candidate-node set, with keyword filtering and grouped results.",
      },
    },
  },
  args: {
    open: true,
    session: sessionsFixture.sessions[0] ?? null,
    isPending: false,
    error: null,
    onOpenChange: fn(),
    onSearch: fn(async () => sessionIpNodeOptionsFixture.groups),
    onSubmit: fn(),
  },
} satisfies Meta<typeof SessionNodeSelectDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
  async play({ canvasElement }) {
    const dialog = within(canvasElement.ownerDocument.body);
    await expect(
      await dialog.findByPlaceholderText(/Search IP, node, subscription, or city/i),
    ).toBeInTheDocument();
    await expect((await dialog.findAllByText(/203\.0\.113\.10/i)).length).toBeGreaterThan(0);
    await expect(await dialog.findByText(/2 candidate nodes selected/i)).toBeInTheDocument();
    await expect((await dialog.findAllByText(/JP-Tokyo-Entry/i)).length).toBeGreaterThan(0);
    const latencyLabels = await dialog.findAllByText(/88 ms/i);
    await expect(latencyLabels.length).toBeGreaterThan(0);
    const firstLatencyLabel = latencyLabels[0];
    if (!firstLatencyLabel) {
      throw new Error("Expected an 88 ms latency label");
    }
    await userEvent.hover(firstLatencyLabel);
    await expect(
      (await dialog.findAllByText(/Latency quality: Excellent/i)).length,
    ).toBeGreaterThan(0);
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

export const LoadFailure: Story = {
  args: {
    onSearch: fn(async () => {
      throw new Error("network unavailable");
    }),
  },
};
