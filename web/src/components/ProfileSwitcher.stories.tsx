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
    return <ProfileSwitcher profileId={profileId} onProfileIdChange={setProfileId} />;
  },
  args: {
    profileId: "default",
    onProfileIdChange: () => undefined,
  },
} satisfies Meta<typeof ProfileSwitcher>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
