import type { Meta, StoryObj } from "@storybook/react-vite";

import { DataTablePanel } from "@/components/DataTablePanel";

const meta = {
  title: "Components/DataTablePanel",
  component: DataTablePanel,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Shared dense table container with contextual header, request chips, and room for actions.",
      },
    },
  },
  args: {
    eyebrow: "Results",
    title: "Candidate IPs",
    description: "Summarize the current slice before showing the table itself.",
    chips: ["2 rows", "countries: JP, US", "sort: LRU"],
    children: (
      <div className="rounded-2xl border border-dashed border-border/70 p-6 text-sm text-muted-foreground">
        Table content goes here.
      </div>
    ),
  },
} satisfies Meta<typeof DataTablePanel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
