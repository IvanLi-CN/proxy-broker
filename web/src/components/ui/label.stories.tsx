import type { Meta, StoryObj } from "@storybook/react-vite";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

const meta = {
  title: "UI/Label",
  component: Label,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Field label primitive paired with form controls across the app.",
      },
    },
  },
} satisfies Meta<typeof Label>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <div className="grid max-w-sm gap-2">
      <Label htmlFor="story-label">Profile ID</Label>
      <Input id="story-label" defaultValue="default" />
    </div>
  ),
};
