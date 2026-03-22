import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { TasksTable } from "@/features/tasks/components/TasksTable";
import { tasksFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Tasks/TasksTable",
  component: TasksTable,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Dense run board table for live task inventory, selection, and quick status scanning.",
      },
    },
  },
  args: {
    runs: tasksFixture.runs,
    isLoading: false,
    selectedRunId: tasksFixture.runs[0]?.run_id ?? null,
    onSelectRun: fn(),
  },
} satisfies Meta<typeof TasksTable>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
