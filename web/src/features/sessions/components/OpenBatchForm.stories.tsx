import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { OpenBatchForm } from "@/features/sessions/components/OpenBatchForm";
import { batchFixture } from "@/mocks/fixtures";

const searchOptions = async ({
  kind,
  country_codes,
}: {
  kind: "country" | "city" | "ip";
  country_codes?: string[];
}) => {
  if (kind === "country") {
    return [
      { value: "JP", label: "Japan (JP)", meta: "Japan" },
      { value: "US", label: "United States (US)", meta: "United States" },
    ];
  }
  if (kind === "city") {
    if (country_codes?.includes("JP")) {
      return [{ value: "Tokyo", label: "Tokyo", meta: "Japan (JP)" }];
    }
    return [{ value: "San Jose", label: "San Jose", meta: "United States (US)" }];
  }
  return [
    { value: "203.0.113.10", label: "203.0.113.10", meta: "JP / Chiyoda" },
    { value: "203.0.113.88", label: "203.0.113.88", meta: "JP / Osaka" },
    { value: "198.51.100.42", label: "198.51.100.42", meta: "US / San Jose" },
  ];
};

const meta = {
  title: "Features/Sessions/OpenBatchForm",
  component: OpenBatchForm,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Transactional batch opener that reuses the same three-mode targeting model per row and keeps exclusions inside an Advanced panel.",
      },
    },
  },
  args: {
    isPending: false,
    onSubmit: fn(),
    response: batchFixture,
    error: null,
    suggestedPort: 10080,
    searchOptions,
  },
} satisfies Meta<typeof OpenBatchForm>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const GeoRow: Story = {
  args: {
    response: null,
    initialRequests: [
      {
        selectionMode: "geo",
        desiredPort: "",
        countryCodes: ["JP"],
        cities: ["Tokyo"],
        specifiedIps: [],
        excludedIps: [],
        sortMode: "lru",
      },
    ],
  },
};

export const AdvancedOpen: Story = {
  args: {
    response: null,
    defaultAdvancedOpen: true,
    initialRequests: [
      {
        selectionMode: "ip",
        desiredPort: "",
        countryCodes: [],
        cities: [],
        specifiedIps: ["203.0.113.10"],
        excludedIps: ["198.51.100.42"],
        sortMode: "mru",
      },
    ],
  },
};

export const Interaction: Story = {
  args: {
    response: null,
    error: null,
    onSubmit: fn(),
  },
  async play({ canvasElement, args }) {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: /add request row/i }));
    await userEvent.click(canvas.getAllByRole("button", { name: /^ip$/i })[1] ?? canvas.getByRole("button", { name: /^ip$/i }));
    await userEvent.click(canvas.getAllByRole("combobox", { name: /ip/i })[0] ?? canvas.getByRole("combobox", { name: /ip/i }));

    const overlay = within(document.body);
    await waitFor(() => expect(overlay.getByText("203.0.113.10")).toBeVisible());
    await userEvent.click(overlay.getByText("203.0.113.10"));
    await userEvent.click(canvas.getByRole("button", { name: /open batch/i }));

    await waitFor(() => {
      expect(args.onSubmit).toHaveBeenCalled();
    });
  },
};
