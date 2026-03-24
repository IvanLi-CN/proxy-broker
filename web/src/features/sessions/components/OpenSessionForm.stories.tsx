import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { OpenSessionForm } from "@/features/sessions/components/OpenSessionForm";
import { sessionFixture } from "@/mocks/fixtures";

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
      return [
        { value: "JP::Tokyo", label: "Tokyo", meta: "Japan (JP)" },
        { value: "JP::Osaka", label: "Osaka", meta: "Japan (JP)" },
      ];
    }
    return [{ value: "US::San Jose", label: "San Jose", meta: "United States (US)" }];
  }
  return [
    { value: "203.0.113.10", label: "203.0.113.10", meta: "JP / Chiyoda" },
    { value: "203.0.113.88", label: "203.0.113.88", meta: "JP / Osaka" },
    { value: "198.51.100.42", label: "198.51.100.42", meta: "US / San Jose" },
  ];
};

const meta = {
  title: "Features/Sessions/OpenSessionForm",
  component: OpenSessionForm,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Single-session opener with a three-mode targeting switch, searchable multi-select inputs, and an Advanced exclude-IP drawer.",
      },
    },
  },
  args: {
    isPending: false,
    onSubmit: fn(),
    response: sessionFixture,
    error: null,
    suggestedPort: 10080,
    searchOptions,
  },
} satisfies Meta<typeof OpenSessionForm>;

export default meta;
type Story = StoryObj<typeof meta>;

export const AnyMode: Story = {};

export const GeoMode: Story = {
  args: {
    response: null,
    initialValues: {
      selectionMode: "geo",
      countryCodes: ["JP"],
      cities: ["Tokyo"],
    },
  },
};

export const IpMode: Story = {
  args: {
    response: null,
    initialValues: {
      selectionMode: "ip",
      specifiedIps: ["203.0.113.10", "203.0.113.88"],
    },
  },
};

export const AdvancedOpen: Story = {
  args: {
    response: null,
    defaultAdvancedOpen: true,
    initialValues: {
      selectionMode: "geo",
      countryCodes: ["JP"],
      excludedIps: ["198.51.100.42"],
    },
  },
};

export const ErrorState: Story = {
  args: {
    response: null,
    error: "invalid_request: selection_mode=geo requires at least one country_codes or cities entry",
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
    await userEvent.click(canvas.getByRole("button", { name: /ip/i }));
    await userEvent.click(canvas.getByRole("combobox", { name: /ip/i }));

    const overlay = within(document.body);
    await waitFor(() => expect(overlay.getByText("203.0.113.10")).toBeVisible());
    await userEvent.click(overlay.getByText("203.0.113.10"));

    await userEvent.clear(canvas.getByLabelText("Desired port"));
    await userEvent.type(canvas.getByLabelText("Desired port"), "10088");
    await userEvent.click(canvas.getByRole("button", { name: /open session/i }));

    await waitFor(() => {
      expect(args.onSubmit).toHaveBeenCalled();
    });
  },
};
