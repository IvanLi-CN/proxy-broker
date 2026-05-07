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
          "Dialog for switching the current session to another node, with keyword filtering, two pinned recency groups, and region/source grouping.",
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
    await expect(await dialog.findByText(/Group nodes/i)).toBeInTheDocument();
    await expect(
      await dialog.findByRole("button", { name: /Current session last used/i }),
    ).toBeInTheDocument();
    await expect((await dialog.findAllByRole("button", { name: /Japan/i })).length).toBeGreaterThan(
      0,
    );
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

export const SourceGrouping: Story = {
  args: {},
  async play({ canvasElement }) {
    const dialog = within(canvasElement.ownerDocument.body);
    await userEvent.click(await dialog.findByRole("radio", { name: /Group by subscription/i }));
    await expect(
      (await dialog.findAllByRole("button", { name: /browser-core/i })).length,
    ).toBeGreaterThan(0);
    const fallbackButtons = await dialog.findAllByRole("button", { name: /fallback-lab/i });
    await userEvent.click(fallbackButtons[0] as HTMLElement);
    await expect(
      await dialog.findByRole("button", { name: /US-SanJose-Edge/i }),
    ).toBeInTheDocument();
  },
};
