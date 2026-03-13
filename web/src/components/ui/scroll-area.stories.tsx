import type { Meta, StoryObj } from "@storybook/react-vite";

import { ScrollArea } from "@/components/ui/scroll-area";

const meta = {
  title: "UI/ScrollArea",
  component: ScrollArea,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Bounded scroll container for long tables and side panes.",
      },
    },
  },
} satisfies Meta<typeof ScrollArea>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <ScrollArea className="h-48 rounded-2xl border border-border/70 bg-card/90 p-4">
      <div className="space-y-3">
        {Array.from({ length: 12 }, (_, index) => `Candidate row #${index + 1}`).map((label) => (
          <div className="rounded-xl border border-border/60 bg-background/80 p-3" key={label}>
            {label}
          </div>
        ))}
      </div>
    </ScrollArea>
  ),
};
