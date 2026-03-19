import type { Meta, StoryObj } from "@storybook/react-vite";

import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";

const meta = {
  title: "Components/UI/Popover",
  component: Popover,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Anchored overlay surface used by selectors and compact control-room menus.",
      },
    },
  },
  render: () => (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant="outline">Open profile menu</Button>
      </PopoverTrigger>
      <PopoverContent>
        <div className="space-y-1.5 p-3 text-sm">
          <div className="font-medium">Catalog overlay</div>
          <div className="text-muted-foreground">
            Use this surface for compact anchored menus and combobox content.
          </div>
        </div>
      </PopoverContent>
    </Popover>
  ),
} satisfies Meta<typeof Popover>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
