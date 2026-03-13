import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const meta = {
  title: "UI/Select",
  component: Select,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Dropdown primitive used for sort modes and source types.",
      },
    },
  },
} satisfies Meta<typeof Select>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => {
    const [value, setValue] = useState("lru");
    return (
      <Select onValueChange={setValue} value={value}>
        <SelectTrigger aria-label="Sort mode" className="w-48">
          <SelectValue placeholder="Sort mode" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="lru">LRU</SelectItem>
          <SelectItem value="mru">MRU</SelectItem>
        </SelectContent>
      </Select>
    );
  },
};
