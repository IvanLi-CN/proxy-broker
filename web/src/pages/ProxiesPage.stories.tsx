import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { AppShell } from "@/components/AppShell";
import { ProxiesPage } from "@/pages/ProxiesPage";

const inventoryFixture = {
  items: [
    {
      node_id: "node-global-1",
      proxy_name: "global-jp-entry",
      proxy_type: "socks5",
      server: "jp.example.com",
      resolved_ips: ["203.0.113.10", "203.0.113.11"],
      source_scope: { type: "global" as const },
      allocation_scope: { type: "global" as const },
      effective_profile_ids: ["default", "edge-jp", "lab-us"],
    },
    {
      node_id: "node-profile-1",
      proxy_name: "edge-jp-local",
      proxy_type: "http",
      server: "edge-jp.internal",
      resolved_ips: ["198.51.100.22"],
      source_scope: { type: "profile" as const, profile_id: "edge-jp" },
      allocation_scope: { type: "profile" as const, profile_id: "edge-jp" },
      effective_profile_ids: ["edge-jp"],
    },
    {
      node_id: "node-reassigned-1",
      proxy_name: "lab-shared",
      proxy_type: "socks5",
      server: "shared.example.com",
      resolved_ips: ["192.0.2.44"],
      source_scope: { type: "profile" as const, profile_id: "lab-us" },
      allocation_scope: { type: "global" as const },
      effective_profile_ids: ["default", "edge-jp", "lab-us"],
    },
  ],
};

const meta = {
  title: "Pages/ProxiesPage",
  component: ProxiesPage,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    initialEntries: ["/proxies"],
    docs: {
      description: {
        component:
          "Administrator proxies workspace dedicated to the shared global pool and cross-profile allocations.",
      },
    },
  },
  render: (args) => (
    <AppShell
      profileId="edge-jp"
      profiles={args.profiles}
      shellMode="global"
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
      <ProxiesPage {...args} />
    </AppShell>
  ),
  args: {
    profiles: ["default", "edge-jp", "lab-us"],
    currentUser: {
      status: "resolved",
      identity: {
        authenticated: true,
        principal_type: "human",
        subject: "admin@example.com",
        email: "admin@example.com",
        groups: ["admins"],
        is_admin: true,
      },
    },
    accessDenied: false,
    authError: null,
    globalLoadResponse: {
      loaded_proxies: 12,
      distinct_ips: 9,
      warnings: [],
    },
    globalLoadError: null,
    loadingGlobal: false,
    inventory: inventoryFixture,
    inventoryLoading: false,
    inventoryError: null,
    reallocatingNodeId: null,
    deletingNodeId: null,
    onLoadGlobal: fn(),
    onReassignNode: fn(),
    onDeleteNode: fn(),
  },
} satisfies Meta<typeof ProxiesPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { name: /proxies/i })).toBeVisible();
    await expect(canvas.getByRole("heading", { name: /import global proxy pool/i })).toBeVisible();
    await expect(
      canvas.getByRole("heading", { name: /global inventory and allocations/i }),
    ).toBeVisible();
    await expect(canvas.getByRole("button", { name: /delete/i })).toBeVisible();
  },
};

export const ZhCN: Story = {
  globals: {
    locale: "zh-CN",
  },
};

export const AccessDenied: Story = {
  args: {
    accessDenied: true,
    globalLoadResponse: null,
    inventory: null,
  },
};
