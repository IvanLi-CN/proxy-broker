import type { Meta, StoryObj } from "@storybook/react-vite";

import { IpResultsTable } from "@/features/ips/components/IpResultsTable";
import { ipResultsFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/IP Extract/IpResultsTable",
  component: IpResultsTable,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Results table for extracted candidates with geo, probe, and last-used metadata plus loading and empty states.",
      },
    },
  },
  args: {
    items: ipResultsFixture.items,
    isLoading: false,
  },
} satisfies Meta<typeof IpResultsTable>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

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
