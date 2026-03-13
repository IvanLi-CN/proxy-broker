import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";

const meta = {
  title: "UI/Checkbox",
  component: Checkbox,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Boolean control used for force-refresh and opt-in behaviors.",
      },
    },
  },
} satisfies Meta<typeof Checkbox>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => {
    const [checked, setChecked] = useState(true);
    return (
      <div className="flex items-start gap-3">
        <Checkbox
          checked={checked}
          id="force"
          onCheckedChange={(value) => setChecked(Boolean(value))}
        />
        <div className="space-y-1">
          <Label htmlFor="force">Force refresh stale cache</Label>
          <p className="text-xs text-muted-foreground">Ignore cache TTL and probe every target.</p>
        </div>
      </div>
    );
  },
};
