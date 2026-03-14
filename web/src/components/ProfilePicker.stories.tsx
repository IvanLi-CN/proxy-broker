import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { ProfilePicker } from "@/components/ProfilePicker";

const meta = {
  title: "Components/ProfilePicker",
  component: ProfilePicker,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Sidebar project selector that mixes known profiles, recent workspaces, and an explicit new-project switch input.",
      },
    },
  },
  render: (args) => {
    const [profileId, setProfileId] = useState(args.activeProfileId);
    return (
      <ProfilePicker
        {...args}
        activeProfileId={profileId}
        onCreateProfileId={(value) => {
          setProfileId(value);
          return true;
        }}
        onSelectProfileId={(value) => {
          setProfileId(value);
        }}
      />
    );
  },
  args: {
    activeProfileId: "default",
    profiles: ["default", "alpha", "beta", "staging-lab"],
    recentProfileIds: ["default", "alpha"],
    isLoading: false,
    isSwitching: false,
    onSelectProfileId: () => undefined,
    onCreateProfileId: () => true,
  },
} satisfies Meta<typeof ProfilePicker>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Switching: Story = {
  args: {
    isSwitching: true,
  },
};
