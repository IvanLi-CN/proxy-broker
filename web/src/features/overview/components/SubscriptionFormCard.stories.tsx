import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { SubscriptionFormCard } from "@/features/overview/components/SubscriptionFormCard";
import { subscriptionFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Overview/SubscriptionFormCard",
  component: SubscriptionFormCard,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Subscription source form that maps directly to the load-subscription REST payload.",
      },
    },
  },
  args: {
    isPending: false,
    onSubmit: fn(),
    response: subscriptionFixture,
    error: null,
  },
} satisfies Meta<typeof SubscriptionFormCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ErrorState: Story = {
  args: {
    response: null,
    error: "subscription_invalid: payload is malformed",
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
    await userEvent.clear(canvas.getByLabelText("Value"));
    await userEvent.type(canvas.getByLabelText("Value"), "https://example.com/feed.yaml");
    await userEvent.click(canvas.getByRole("button", { name: /load subscription/i }));
    expect(args.onSubmit).toHaveBeenCalled();
  },
};
