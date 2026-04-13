import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { ProfileProxyPolicyCard } from "@/features/proxies/components/ProfileProxyPolicyCard";

const meta = {
  title: "Features/Proxies/ProfileProxyPolicyCard",
  component: ProfileProxyPolicyCard,
  tags: ["autodocs"],
  args: {
    profileId: "edge-jp",
    useGlobalProxies: true,
    proxySettingsLoading: false,
    updatingSettings: false,
    proxySettingsError: null,
    onToggleUseGlobalProxies: fn(),
  },
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component:
          "Profile-scoped policy card that only controls whether the current profile inherits the global pool.",
      },
    },
  },
} satisfies Meta<typeof ProfileProxyPolicyCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const LocalOnly: Story = {
  args: {
    useGlobalProxies: false,
  },
};

export const Interaction: Story = {
  args: {
    onToggleUseGlobalProxies: fn(),
  },
  async play({ canvasElement, args }) {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("checkbox", { name: /use global pool for edge-jp/i }));
    expect(args.onToggleUseGlobalProxies).toHaveBeenCalledWith(false);
  },
};
