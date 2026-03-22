import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { taskDetailFixture, tasksFixture } from "@/mocks/fixtures";
import { TasksPage } from "@/pages/TasksPage";

const meta = {
  title: "Pages/TasksPage",
  component: TasksPage,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Task center surface for scheduled subscription sync and metadata refresh monitoring, including SSE-backed list and detail states.",
      },
    },
  },
  args: {
    profileId: "default",
    scope: "current",
    kind: undefined,
    status: undefined,
    trigger: undefined,
    runningOnly: false,
    onScopeChange: fn(),
    onKindChange: fn(),
    onStatusChange: fn(),
    onTriggerChange: fn(),
    onRunningOnlyChange: fn(),
    taskList: tasksFixture,
    tasksLoading: false,
    taskError: null,
    streamState: "live",
    selectedRunId: taskDetailFixture.run.run_id,
    onSelectRun: fn(),
    selectedRunDetail: taskDetailFixture,
    selectedRunLoading: false,
    detailError: null,
    accessDenied: false,
  },
} satisfies Meta<typeof TasksPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Reconnecting: Story = {
  args: {
    streamState: "reconnecting",
  },
};

export const AccessDenied: Story = {
  args: {
    accessDenied: true,
    taskList: null,
    selectedRunDetail: null,
  },
};
