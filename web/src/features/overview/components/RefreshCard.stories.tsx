import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { RefreshCard } from "@/features/overview/components/RefreshCard";
import { refreshFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Overview/RefreshCard",
  component: RefreshCard,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Refresh action card for probe + geo metadata, including a force-refresh toggle.",
      },
    },
  },
  args: {
    isPending: false,
    onSubmit: fn(),
    response: refreshFixture,
    error: null,
  },
} satisfies Meta<typeof RefreshCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ErrorState: Story = {
  args: {
    response: null,
    error: "mihomo_unavailable: runtime is not reachable",
  },
};

export const Interaction: Story = {
  args: {
    response: null,
    error: null,
    onSubmit: fn(),
  },
  async play({ canvasElement, args }) {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByLabelText(/force refresh stale entries/i));
    await userEvent.click(canvas.getByRole("button", { name: /refresh metadata/i }));
    expect(args.onSubmit).toHaveBeenCalled();
  },
};
