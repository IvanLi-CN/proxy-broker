import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { AppShell } from "@/components/AppShell";
import { taskDetailFixture, tasksFixture } from "@/mocks/fixtures";
import { TasksPage } from "@/pages/TasksPage";

const meta = {
  title: "Pages/TasksPage",
  component: TasksPage,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    initialEntries: ["/tasks"],
    docs: {
      description: {
        component:
          "Task center surface inside the real app shell, opening directly on the summary cards, filters, and SSE-backed list/detail split without a route hero.",
      },
    },
  },
  render: (args) => (
    <AppShell
      projectId={args.projectId}
      projects={["default", "edge-jp", "lab-us"]}
      projectsLoading={false}
      projectsCreating={false}
      projectsError={null}
      healthStatus="ok"
      currentUser={{
        status: "resolved",
        identity: {
          authenticated: true,
          principal_type: "human",
          subject: "admin@example.com",
          email: "admin@example.com",
          groups: ["admins", "ops"],
          is_admin: true,
        },
      }}
      onProjectIdChange={() => undefined}
      onCreateProject={async (value: string) => value}
      onRetryProjects={() => undefined}
    >
      <TasksPage {...args} />
    </AppShell>
  ),
  args: {
    projectId: "default",
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

export const ZhCN: Story = {
  globals: {
    locale: "zh-CN",
  },
};

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
