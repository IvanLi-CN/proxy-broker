import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { Toggle } from "@/components/ui/toggle";

const meta = {
  title: "UI/Toggle",
  component: Toggle,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Single shadcn/ui toggle primitive used by the segmented copy-format selector.",
      },
    },
  },
} satisfies Meta<typeof Toggle>;

export default meta;
type Story = StoryObj<typeof meta>;

function TogglePreview() {
  const [pressed, setPressed] = useState(false);

  return (
    <div className="flex flex-wrap items-center gap-3">
      <Toggle pressed={pressed} onPressedChange={setPressed}>
        {pressed ? "Enabled" : "Toggle me"}
      </Toggle>
      <Toggle pressed variant="outline">
        Selected
      </Toggle>
      <Toggle disabled>Disabled</Toggle>
    </div>
  );
}

export const Default: Story = {
  render: () => <TogglePreview />,
};
