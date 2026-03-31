import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { NodesTable } from "@/features/nodes/components/NodesTable";
import { nodesFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Nodes/NodesTable",
  component: NodesTable,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Dense node inventory table with per-page selection and local grouping headers for the current result page.",
      },
    },
  },
  args: {
    items: nodesFixture.items,
    isLoading: false,
    viewMode: "flat",
    selectedIds: [],
    onToggleSelect: fn(),
    onToggleSelectAll: fn(),
  },
} satisfies Meta<typeof NodesTable>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const GroupedByIp: Story = {
  args: {
    viewMode: "group_by_ip",
  },
};

export const Loading: Story = {
  args: {
    items: [],
    isLoading: true,
  },
};

export const Empty: Story = {
  args: {
    items: [],
    isLoading: false,
  },
};
