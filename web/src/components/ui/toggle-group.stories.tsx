import type { Meta, StoryObj } from "@storybook/react-vite";

import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

const meta = {
  title: "UI/ToggleGroup",
  component: ToggleGroup,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Single-select shadcn/ui segmented control used for the sessions copy-format switcher.",
      },
    },
  },
  args: {
    type: "single",
  },
} satisfies Meta<typeof ToggleGroup>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <div className="flex max-w-xl items-center gap-3">
      <span className="shrink-0 text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
        Copy format
      </span>
      <ToggleGroup
        type="single"
        defaultValue="socks_url"
        aria-label="Copy format"
        className="min-w-0 flex-1 rounded-full border border-border/70 bg-background/80 p-1"
      >
        <ToggleGroupItem
          value="socks_url"
          className="flex-1 rounded-full border-0 px-3 py-2 text-xs font-semibold text-muted-foreground shadow-none transition-colors hover:text-foreground data-[state=on]:bg-card data-[state=on]:text-foreground data-[state=on]:shadow-sm sm:text-[13px]"
        >
          SOCKS URI
        </ToggleGroupItem>
        <ToggleGroupItem
          value="http_url"
          className="flex-1 rounded-full border-0 px-3 py-2 text-xs font-semibold text-muted-foreground shadow-none transition-colors hover:text-foreground data-[state=on]:bg-card data-[state=on]:text-foreground data-[state=on]:shadow-sm sm:text-[13px]"
        >
          HTTP URI
        </ToggleGroupItem>
        <ToggleGroupItem
          value="host_port"
          className="flex-1 rounded-full border-0 px-3 py-2 text-xs font-semibold text-muted-foreground shadow-none transition-colors hover:text-foreground data-[state=on]:bg-card data-[state=on]:text-foreground data-[state=on]:shadow-sm sm:text-[13px]"
        >
          Host:port
        </ToggleGroupItem>
      </ToggleGroup>
    </div>
  ),
};
