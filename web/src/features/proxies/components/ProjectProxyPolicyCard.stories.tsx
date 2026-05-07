import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { ProjectProxyPolicyCard } from "@/features/proxies/components/ProjectProxyPolicyCard";

const meta = {
  title: "Features/Proxies/ProjectProxyPolicyCard",
  component: ProjectProxyPolicyCard,
  tags: ["autodocs"],
  args: {
    projectId: "edge-jp",
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
          "Project-scoped policy card that only controls whether the current project inherits the global pool.",
      },
    },
  },
} satisfies Meta<typeof ProjectProxyPolicyCard>;

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
