import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { SearchableMultiSelect } from "@/components/SearchableMultiSelect";

const options = [
  { value: "JP", label: "Japan (JP)", meta: "Asia" },
  { value: "US", label: "United States (US)", meta: "North America" },
  { value: "SG", label: "Singapore (SG)", meta: "Asia" },
];

const meta = {
  title: "Components/SearchableMultiSelect",
  component: SearchableMultiSelect,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Popover + Command based searchable multi-select used by the Sessions targeting controls.",
      },
    },
  },
  args: {
    id: "storybook-searchable-multi-select",
    label: "Countries",
    helper: "Search and select multiple options.",
    placeholder: "Choose countries",
    searchPlaceholder: "Search countries",
    emptyText: "No matching countries",
    values: [],
    onChange: fn(),
    onSearch: async () => options,
  },
} satisfies Meta<typeof SearchableMultiSelect>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const WithValues: Story = {
  args: {
    values: ["JP", "SG"],
  },
};

export const Interaction: Story = {
  args: {
    onChange: fn(),
  },
  async play({ canvasElement, args }) {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("combobox", { name: /countries/i }));

    const overlay = within(document.body);
    await waitFor(() => expect(overlay.getByText("Japan (JP)")).toBeVisible());
    await userEvent.click(overlay.getByText("Japan (JP)"));

    await waitFor(() => {
      expect(args.onChange).toHaveBeenCalled();
    });
  },
};
