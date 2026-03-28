import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { AppShell } from "@/components/AppShell";
import { ipResultsFixture } from "@/mocks/fixtures";
import { IpExtractPage } from "@/pages/IpExtractPage";

const meta = {
  title: "Pages/IpExtractPage",
  component: IpExtractPage,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    initialEntries: ["/ips"],
    docs: {
      description: {
        component:
          "IP extraction route inside the real app shell, starting directly on the filter form and candidate deck without a route hero.",
      },
    },
  },
  render: (args) => (
    <AppShell
      profileId="default"
      profiles={["default", "edge-jp", "lab-us"]}
      profilesLoading={false}
      profilesCreating={false}
      profilesError={null}
      healthStatus="ok"
      currentUser={{
        status: "resolved",
        identity: {
          authenticated: true,
          principal_type: "human",
          subject: "admin@example.com",
          email: "admin@example.com",
          groups: ["admins", "ops"],
          is_admin: true,
        },
      }}
      onProfileIdChange={() => undefined}
      onCreateProfile={async (value: string) => value}
      onRetryProfiles={() => undefined}
    >
      <IpExtractPage {...args} />
    </AppShell>
  ),
  args: {
    isPending: false,
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

export const ZhCN: Story = {
  globals: {
    locale: "zh-CN",
  },
};

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
