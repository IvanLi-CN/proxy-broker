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
          "Results table for extracted candidates with geo, probe, and last-used metadata.",
      },
    },
  },
  args: {
    items: ipResultsFixture.items,
  },
} satisfies Meta<typeof IpResultsTable>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Empty: Story = {
  args: {
    items: [],
  },
};
