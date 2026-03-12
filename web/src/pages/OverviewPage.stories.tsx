import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "@storybook/test";
import {
  healthFixture,
  refreshFixture,
  sessionsFixture,
  subscriptionFixture,
} from "@/mocks/fixtures";
import { OverviewPage } from "@/pages/OverviewPage";

const meta = {
  title: "Pages/OverviewPage",
  component: OverviewPage,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Overview route composition for health, subscription loading, refresh actions, and active session summary.",
      },
    },
  },
  args: {
    health: healthFixture,
    activeSessions: sessionsFixture.sessions.length,
    loadResponse: subscriptionFixture,
    loadError: null,
    refreshResponse: refreshFixture,
    refreshError: null,
    loadingSubscription: false,
    refreshing: false,
    onLoadSubscription: fn(),
    onRefresh: fn(),
  },
} satisfies Meta<typeof OverviewPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ErrorState: Story = {
  args: {
    loadResponse: null,
    loadError: "subscription_invalid: malformed upstream payload",
    refreshResponse: null,
    refreshError: "mihomo_unavailable: controller not reachable",
  },
};
