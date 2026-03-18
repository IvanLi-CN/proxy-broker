import type { Meta, StoryObj } from "@storybook/react-vite";
import { CheckIcon } from "lucide-react";

import {
  Command,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from "@/components/ui/command";

const meta = {
  title: "Components/UI/Command",
  component: Command,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Searchable command surface used inside anchored menus such as the profile combobox.",
      },
    },
  },
  render: () => (
    <div className="max-w-sm rounded-2xl border border-border bg-card">
      <Command shouldFilter={false}>
        <CommandInput placeholder="Search profiles..." />
        <CommandList>
          <CommandGroup heading="Known profiles">
            <CommandItem value="default">
              <CheckIcon className="size-4 text-primary" />
              <span className="font-mono">default</span>
              <CommandShortcut>Active</CommandShortcut>
            </CommandItem>
            <CommandItem value="edge-jp">
              <span className="size-4" />
              <span className="font-mono">edge-jp</span>
            </CommandItem>
          </CommandGroup>
          <CommandSeparator />
          <CommandGroup heading="Create">
            <CommandItem value="create:fresh-lab">
              <span className="font-medium">Create "fresh-lab"</span>
            </CommandItem>
          </CommandGroup>
        </CommandList>
      </Command>
    </div>
  ),
} satisfies Meta<typeof Command>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
