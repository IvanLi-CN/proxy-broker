import type { Meta, StoryObj } from "@storybook/react-vite";

import { AppShell } from "@/components/AppShell";

const meta = {
  title: "Components/AppShell",
  component: AppShell,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          "Primary application chrome with profile picker, sidebar navigation, and top status rail.",
      },
    },
  },
  render: (args) => (
    <AppShell {...args}>
      <div className="rounded-3xl border border-border/70 bg-card/90 p-8 text-sm text-muted-foreground">
        Routed content renders here.
      </div>
    </AppShell>
  ),
  args: {
    profileId: "default",
    knownProfiles: ["default", "alpha", "beta"],
    recentProfileIds: ["default", "alpha"],
    healthStatus: "ok",
    onProfileIdChange: () => true,
    profileLoading: false,
    profilesLoading: false,
  },
} satisfies Meta<typeof AppShell>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const LoadingWorkspace: Story = {
  args: {
    profileLoading: true,
  },
};
