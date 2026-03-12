import type { Meta, StoryObj } from "@storybook/react-vite";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

const meta = {
  title: "UI/Tooltip",
  component: Tooltip,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Hover/focus affordance used for icon-only controls and compact hints.",
      },
    },
  },
} satisfies Meta<typeof Tooltip>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button size="icon-sm" variant="outline">
          ?
        </Button>
      </TooltipTrigger>
      <TooltipContent>Operators can collapse the sidebar with Cmd/Ctrl+B.</TooltipContent>
    </Tooltip>
  ),
};
