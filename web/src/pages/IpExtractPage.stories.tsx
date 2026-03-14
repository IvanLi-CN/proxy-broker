import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { ipResultsFixture } from "@/mocks/fixtures";
import { IpExtractPage } from "@/pages/IpExtractPage";

const meta = {
  title: "Pages/IpExtractPage",
  component: IpExtractPage,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "IP extraction route that pairs the filter builder with a dense candidate deck and request summary chips.",
      },
    },
  },
  args: {
    isPending: false,
    initialized: true,
    initializationLoading: false,
    profileId: "default",
    filtersFormValues: {
      countryCodes: "JP, US",
      cities: "Tokyo",
      specifiedIps: "",
      blacklistIps: "",
      limit: "20",
      sortMode: "lru",
    },
    onFormValuesChange: fn(),
    response: ipResultsFixture,
    error: null,
    lastRequest: {
      country_codes: ["JP", "US"],
      cities: ["Tokyo"],
      limit: 20,
      sort_mode: "lru",
    },
    onSubmit: fn(),
  },
} satisfies Meta<typeof IpExtractPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Loading: Story = {
  args: {
    response: null,
    isPending: true,
  },
};

export const ErrorState: Story = {
  args: {
    response: null,
    error: "ip_conflict_blacklist: the same IP appears in both include and blacklist lists",
  },
};

export const UninitializedProject: Story = {
  args: {
    initialized: false,
    response: null,
  },
};
