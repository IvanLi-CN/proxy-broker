import type { Meta, StoryObj } from "@storybook/react-vite";

import { Textarea } from "@/components/ui/textarea";

const meta = {
  title: "UI/Textarea",
  component: Textarea,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Multi-line text input for comma/newline separated IP and geo lists.",
      },
    },
  },
  args: {
    placeholder: "JP, US, SG",
  },
} satisfies Meta<typeof Textarea>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const FieldSizes: Story = {
  render: (args) => (
    <div className="grid max-w-xl gap-4">
      <Textarea {...args} size="sm" placeholder="Small multiline field" />
      <Textarea {...args} placeholder="Default multiline field" />
      <Textarea {...args} size="lg" placeholder="Large multiline field" />
    </div>
  ),
};
