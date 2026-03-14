import type { Meta, StoryObj } from "@storybook/react-vite";

import { Input } from "@/components/ui/input";

const meta = {
  title: "UI/Input",
  component: Input,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Single-line input used for profile IDs, ports, and fixed strings.",
      },
    },
  },
  args: {
    placeholder: "127.0.0.1:10080",
  },
} satisfies Meta<typeof Input>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const FieldSizes: Story = {
  render: (args) => (
    <div className="grid max-w-md gap-4">
      <Input {...args} size="sm" placeholder="Small field" />
      <Input {...args} placeholder="Default field" />
      <Input {...args} size="lg" placeholder="Large field" />
    </div>
  ),
};
