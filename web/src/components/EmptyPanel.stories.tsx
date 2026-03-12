import type { Meta, StoryObj } from "@storybook/react-vite";
import { InboxIcon } from "lucide-react";

import { EmptyPanel } from "@/components/EmptyPanel";

const meta = {
  title: "Components/EmptyPanel",
  component: EmptyPanel,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Fallback card shown when a data region has no current records to display.",
      },
    },
  },
  args: {
    title: "No active sessions",
    description: "Open a single session or a batch to populate this table.",
    icon: InboxIcon,
  },
} satisfies Meta<typeof EmptyPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
