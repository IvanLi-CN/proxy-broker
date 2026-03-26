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
      { value: "JP", label: "日本 (JP)", meta: "日本" },
      { value: "US", label: "美国 (US)", meta: "美国" },
    ];
  }
  if (kind === "city") {
    if (country_codes?.includes("JP")) {
      return [{ value: "JP::Tokyo", label: "东京", meta: "日本 (JP)" }];
    }
    return [{ value: "US::San Jose", label: "圣何塞", meta: "美国 (US)" }];
  }
  return [
    { value: "203.0.113.10", label: "203.0.113.10", meta: "日本 / 千代田" },
    { value: "203.0.113.88", label: "203.0.113.88", meta: "日本 / 大阪" },
    { value: "198.51.100.42", label: "198.51.100.42", meta: "美国 / 圣何塞" },
  ];
};

const meta = {
  title: "Features/Sessions/OpenBatchForm",
  component: OpenBatchForm,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          "Transactional batch opener that reuses the same three-mode targeting model per row and keeps exclusions inside an Advanced panel.",
      },
    },
  },
  globals: {
    locale: "zh-CN",
  },
  decorators: [
    (Story) => (
      <div className="mx-auto w-full max-w-[860px] p-6">
        <Story />
      </div>
    ),
  ],
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
    initialRequests: [
      {
        selectionMode: "any",
        desiredPort: "",
        countryCodes: [],
        cities: [],
        specifiedIps: [],
        excludedIps: [],
        sortMode: "lru",
      },
    ],
  },
  async play({ canvasElement, args }) {
    const canvas = within(canvasElement);
    await userEvent.click(
      canvas.getAllByRole("tab", { name: /^ip$/i })[0] ??
        canvas.getByRole("tab", { name: /^ip$/i }),
    );
    await userEvent.click(
      canvas.getAllByRole("combobox", { name: /^ip$/i })[0] ??
        canvas.getByRole("combobox", { name: /^ip$/i }),
    );

    const overlay = within(document.body);
    await waitFor(() => expect(overlay.getByText("203.0.113.10")).toBeVisible());
    await userEvent.click(overlay.getByText("203.0.113.10"));
    await userEvent.click(canvas.getByRole("button", { name: /open batch|打开批次/i }));

    await waitFor(() => {
      expect(args.onSubmit).toHaveBeenCalled();
    });
  },
};
