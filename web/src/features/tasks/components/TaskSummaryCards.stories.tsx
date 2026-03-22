import type { Meta, StoryObj } from "@storybook/react-vite";

import { TaskSummaryCards } from "@/features/tasks/components/TaskSummaryCards";
import { tasksFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Tasks/TaskSummaryCards",
  component: TaskSummaryCards,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Top-line task monitoring cards for stream health, queue depth, failures, and recent run timing.",
      },
    },
  },
  args: {
    summary: tasksFixture.summary,
    streamState: "live",
  },
} satisfies Meta<typeof TaskSummaryCards>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
