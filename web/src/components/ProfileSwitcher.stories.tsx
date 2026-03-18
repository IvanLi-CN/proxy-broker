import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { ProfileSwitcher } from "@/components/ProfileSwitcher";

const meta = {
  title: "Components/ProfileSwitcher",
  component: ProfileSwitcher,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Sidebar profile selector that scopes all API calls to the active profile identifier.",
      },
    },
  },
  render: (args) => {
    const [profileId, setProfileId] = useState(args.profileId);
    return (
      <div className="max-w-sm">
        <ProfileSwitcher
          {...args}
          profileId={profileId}
          onProfileIdChange={setProfileId}
          onCreateProfile={async (value) => {
            setProfileId(value);
            return value;
          }}
        />
      </div>
    );
  },
  args: {
    profileId: "default",
    profiles: ["default", "edge-jp", "lab-us"],
    isLoading: false,
    isCreating: false,
    loadError: null,
    onProfileIdChange: () => undefined,
    onCreateProfile: async (value: string) => value,
    onRetryProfiles: () => undefined,
  },
} satisfies Meta<typeof ProfileSwitcher>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Populated: Story = {};

export const SearchNoMatch: Story = {
  args: {
    profiles: ["default"],
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("combobox"));
    await userEvent.type(canvas.getByPlaceholderText("Search profiles or type a new ID"), "tokyo");
  },
};

export const Creating: Story = {
  args: {
    isCreating: true,
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("combobox"));
    await userEvent.type(
      canvas.getByPlaceholderText("Search profiles or type a new ID"),
      "fresh-lab",
    );
  },
};
