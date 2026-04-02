import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { NodesFiltersBar } from "@/features/nodes/components/NodesFiltersBar";
import { defaultNodeFilterState } from "@/lib/nodes-view";

const meta = {
  title: "Features/Nodes/NodesFiltersBar",
  component: NodesFiltersBar,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Server-side node query controls for search, geo filters, probe state, family filters, and sorting.",
      },
    },
  },
  args: {
    state: defaultNodeFilterState,
    onChange: fn(),
    onReset: fn(),
  },
} satisfies Meta<typeof NodesFiltersBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Populated: Story = {
  args: {
    state: {
      ...defaultNodeFilterState,
      query: "tokyo",
      countryCodes: "JP, US",
      probeStatus: "reachable",
      sortBy: "latency",
      sortOrder: "asc",
    },
  },
};
