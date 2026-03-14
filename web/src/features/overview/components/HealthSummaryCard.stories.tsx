import type { Meta, StoryObj } from "@storybook/react-vite";

import { HealthSummaryCard } from "@/features/overview/components/HealthSummaryCard";

const meta = {
  title: "Features/Overview/HealthSummaryCard",
  component: HealthSummaryCard,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Top-level metric strip for service health, active listeners, pool inventory, and queued operator attention.",
      },
    },
  },
  args: {
    status: "ok",
    activeSessions: 2,
    hasWarnings: false,
    loadedProxies: 48,
    refreshedIps: 26,
  },
} satisfies Meta<typeof HealthSummaryCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const WarningState: Story = {
  args: {
    hasWarnings: true,
    status: "stale",
    refreshedIps: null,
  },
};
