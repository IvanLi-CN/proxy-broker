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
          "Top-level summary card group for service health, active sessions, and operator warnings.",
      },
    },
  },
  args: {
    status: "ok",
    activeSessions: 2,
    hasWarnings: false,
  },
} satisfies Meta<typeof HealthSummaryCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const WarningState: Story = {
  args: {
    hasWarnings: true,
    status: "stale",
  },
};
