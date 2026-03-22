import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { TaskFiltersBar } from "@/features/tasks/components/TaskFiltersBar";

const meta = {
  title: "Features/Tasks/TaskFiltersBar",
  component: TaskFiltersBar,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Compact filter strip for scope, task kind, status, trigger, and running-only toggles.",
      },
    },
  },
  args: {
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
  },
} satisfies Meta<typeof TaskFiltersBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
