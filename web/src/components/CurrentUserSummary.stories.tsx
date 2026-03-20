import type { Meta, StoryObj } from "@storybook/react-vite";

import { CurrentUserSummary } from "@/components/CurrentUserSummary";

const meta = {
  title: "Components/CurrentUserSummary",
  component: CurrentUserSummary,
  tags: ["autodocs"],
  args: {
    currentUser: {
      status: "resolved",
      identity: {
        authenticated: true,
        principal_type: "human",
        subject: "admin@example.com",
        email: "admin@example.com",
        groups: ["proxy-broker-admins", "ops"],
        is_admin: true,
      },
    },
    variant: "detail",
  },
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component:
          "Curated identity status component used by the shell and overview page. It explicitly covers anonymous, loading, error, human, development, and machine-key states.",
      },
    },
  },
} satisfies Meta<typeof CurrentUserSummary>;

export default meta;

type Story = StoryObj<typeof meta>;

export const StateGallery: Story = {
  render: () => (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
      <CurrentUserSummary currentUser={{ status: "loading" }} />
      <CurrentUserSummary currentUser={{ status: "anonymous" }} />
      <CurrentUserSummary
        currentUser={{
          status: "error",
          message: "http_503: auth status temporarily unavailable",
        }}
      />
      <CurrentUserSummary
        currentUser={{
          status: "resolved",
          identity: {
            authenticated: true,
            principal_type: "human",
            subject: "admin@example.com",
            email: "admin@example.com",
            groups: ["proxy-broker-admins", "ops"],
            is_admin: true,
          },
        }}
      />
      <CurrentUserSummary
        currentUser={{
          status: "resolved",
          identity: {
            authenticated: true,
            principal_type: "human",
            subject: "viewer@example.com",
            email: "viewer@example.com",
            groups: ["proxy-broker-viewers"],
            is_admin: false,
          },
        }}
      />
      <CurrentUserSummary
        currentUser={{
          status: "resolved",
          identity: {
            authenticated: true,
            principal_type: "development",
            subject: "dev@local",
            email: "dev@local",
            groups: ["proxy-broker-dev-admin"],
            is_admin: true,
          },
        }}
      />
      <CurrentUserSummary
        currentUser={{
          status: "resolved",
          identity: {
            authenticated: true,
            principal_type: "api_key",
            subject: "smoke-bot",
            groups: [],
            is_admin: false,
            profile_id: "edge-jp",
            api_key_id: "key-42",
          },
        }}
      />
    </div>
  ),
};

export const Anonymous: Story = {
  args: {
    currentUser: {
      status: "anonymous",
    },
  },
};

export const Loading: Story = {
  args: {
    currentUser: {
      status: "loading",
    },
  },
};

export const ErrorState: Story = {
  args: {
    currentUser: {
      status: "error",
      message: "http_503: auth status temporarily unavailable",
    },
  },
};

export const HumanAdmin: Story = {};

export const HumanNonAdmin: Story = {
  args: {
    currentUser: {
      status: "resolved",
      identity: {
        authenticated: true,
        principal_type: "human",
        subject: "viewer@example.com",
        email: "viewer@example.com",
        groups: ["proxy-broker-viewers"],
        is_admin: false,
      },
    },
  },
};

export const DevelopmentAdmin: Story = {
  args: {
    currentUser: {
      status: "resolved",
      identity: {
        authenticated: true,
        principal_type: "development",
        subject: "dev@local",
        email: "dev@local",
        groups: ["proxy-broker-dev-admin"],
        is_admin: true,
      },
    },
  },
};

export const ApiKeyMachine: Story = {
  args: {
    currentUser: {
      status: "resolved",
      identity: {
        authenticated: true,
        principal_type: "api_key",
        subject: "deploy-bot",
        groups: [],
        is_admin: false,
        profile_id: "default",
        api_key_id: "key-7",
      },
    },
  },
};

export const CompactAnonymous: Story = {
  args: {
    currentUser: {
      status: "anonymous",
    },
    variant: "compact",
  },
};

export const CompactAdmin: Story = {
  args: {
    variant: "compact",
  },
};
