import type { Meta, StoryObj } from "@storybook/react-vite";
import { ActivityIcon } from "lucide-react";

import { TopMetricCard } from "@/components/TopMetricCard";

const meta = {
  title: "Components/TopMetricCard",
  component: TopMetricCard,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Compact KPI card used in page headers to summarize live service state and operator cues.",
      },
    },
  },
  args: {
    title: "Service",
    value: "OK",
    description: "Derived from /healthz and refreshed every 10s.",
    icon: ActivityIcon,
    tone: "positive",
  },
} satisfies Meta<typeof TopMetricCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Warning: Story = {
  args: {
    value: "REVIEW",
    tone: "warning",
  },
};
