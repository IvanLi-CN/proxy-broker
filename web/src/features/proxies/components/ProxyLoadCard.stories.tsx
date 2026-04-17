import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { ProxyLoadCard } from "@/features/proxies/components/ProxyLoadCard";

const meta = {
  title: "Features/Proxies/ProxyLoadCard",
  component: ProxyLoadCard,
  tags: ["autodocs"],
  args: {
    eyebrow: "Current profile",
    title: "Import local pool for edge-jp",
    description:
      "Import nodes for the current profile only. These nodes stay local unless you later reassign them from the global inventory.",
    scopeChip: "allocation defaults to edge-jp",
    pending: false,
    response: {
      loaded_proxies: 6,
      distinct_ips: 4,
      warnings: [],
    },
    error: null,
    defaultValue: "https://example.com/profile-subscription.yaml",
    submitLabel: "Import profile pool",
    successTitle: "Profile pool updated",
    successDescription: "Imported 6 proxies across 4 distinct IPs into profile edge-jp.",
    onSubmit: fn(),
  },
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component:
          "Compact subscription import card used by both the global proxies workspace and profile-scoped local import surfaces.",
      },
    },
  },
} satisfies Meta<typeof ProxyLoadCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ErrorState: Story = {
  args: {
    response: null,
    error: "subscription_fetch_failed: upstream not reachable",
  },
};

export const NodeGroupMode: Story = {
  args: {
    response: null,
    error: null,
    onSubmit: fn(),
  },
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("tab", { name: /nodes/i }));
    await expect(await canvas.findByLabelText(/nodes content/i)).toBeVisible();
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
    await userEvent.clear(canvas.getByLabelText("Name"));
    await userEvent.type(canvas.getByLabelText("Name"), "edge-jp");
    await userEvent.clear(canvas.getByLabelText("Value"));
    await userEvent.type(canvas.getByLabelText("Value"), "https://example.com/feed.yaml");
    await userEvent.click(canvas.getByRole("button", { name: /import profile pool/i }));
    expect(args.onSubmit).toHaveBeenCalled();
  },
};
