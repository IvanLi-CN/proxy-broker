import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { SessionSelectionModeSwitch } from "@/features/sessions/components/SessionSelectionModeSwitch";
import type { SessionSelectionMode } from "@/lib/types";

const meta = {
  title: "Features/Sessions/SessionSelectionModeSwitch",
  component: SessionSelectionModeSwitch,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          "Compact segmented control for the three session targeting modes. Built from shadcn Tabs primitives instead of custom card buttons.",
      },
    },
  },
  decorators: [
    (Story) => (
      <div className="mx-auto w-full max-w-[460px] p-6">
        <Story />
      </div>
    ),
  ],
  args: {
    value: "any",
    onChange: () => undefined,
  },
  render: (args) => {
    const [value, setValue] = useState<SessionSelectionMode>(args.value);
    return (
      <SessionSelectionModeSwitch
        {...args}
        value={value}
        onChange={(nextValue) => {
          setValue(nextValue);
          args.onChange(nextValue);
        }}
      />
    );
  },
} satisfies Meta<typeof SessionSelectionModeSwitch>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};

export const WithError: Story = {
  args: {
    value: "geo",
  },
};
