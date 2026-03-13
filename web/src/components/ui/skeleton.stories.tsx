import type { Meta, StoryObj } from "@storybook/react-vite";

import { Skeleton } from "@/components/ui/skeleton";

const meta = {
  title: "UI/Skeleton",
  component: Skeleton,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Loading placeholder for cards, rows, and async panel states.",
      },
    },
  },
} satisfies Meta<typeof Skeleton>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <div className="grid gap-3">
      <Skeleton className="h-10 w-2/3 rounded-xl" />
      <Skeleton className="h-28 w-full rounded-2xl" />
    </div>
  ),
};
