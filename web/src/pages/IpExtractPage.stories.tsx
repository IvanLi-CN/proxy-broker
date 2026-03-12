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
          "IP extraction route that pairs the filter form with the resulting candidate table and operator guidance.",
      },
    },
  },
  args: {
    isPending: false,
    response: ipResultsFixture,
    error: null,
    onSubmit: fn(),
  },
} satisfies Meta<typeof IpExtractPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ErrorState: Story = {
  args: {
    response: null,
    error: "ip_conflict_blacklist: the same IP appears in both include and blacklist lists",
  },
};
