import type { Meta, StoryObj } from "@storybook/react-vite";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";

const meta = {
  title: "Components/ActionResponsePanel",
  component: ActionResponsePanel,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Inline result banner used to show successful runs, warnings, and operator-facing failures.",
      },
    },
  },
  args: {
    title: "Subscription loaded",
    description: "Loaded 48 proxies across 26 distinct IPs.",
    tone: "success",
  },
} satisfies Meta<typeof ActionResponsePanel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Warning: Story = {
  args: {
    title: "Subscription warnings",
    tone: "warning",
    description: "The backend finished but some DNS resolutions fell back to cached IPs.",
    bullets: ["JP-Relay-02 reused 1 cached IP", "SG-Relay-01 reused 2 cached IPs"],
  },
};

export const ErrorState: Story = {
  args: {
    title: "Open failed",
    tone: "error",
    description: "invalid_port: desired_port must be between 1 and 65535",
  },
};
