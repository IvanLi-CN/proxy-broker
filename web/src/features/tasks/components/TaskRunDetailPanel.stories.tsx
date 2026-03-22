import type { Meta, StoryObj } from "@storybook/react-vite";

import { TaskRunDetailPanel } from "@/features/tasks/components/TaskRunDetailPanel";
import { taskDetailFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Tasks/TaskRunDetailPanel",
  component: TaskRunDetailPanel,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Right-rail run detail surface with summary payloads, failure context, and chronological event logs.",
      },
    },
  },
  args: {
    detail: taskDetailFixture,
    isLoading: false,
  },
} satisfies Meta<typeof TaskRunDetailPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const EmptyState: Story = {
  args: {
    detail: null,
  },
};
