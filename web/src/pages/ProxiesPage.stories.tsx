import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { AppShell } from "@/components/AppShell";
import { GLOBAL_PROFILE_ID } from "@/lib/profile-selection";
import { ProxiesPage } from "@/pages/ProxiesPage";

const profiles = ["default", "edge-jp", "lab-us"];

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
  ],
};

const currentUser = {
  status: "resolved" as const,
  identity: {
    authenticated: true,
    principal_type: "human" as const,
    subject: "admin@example.com",
    email: "admin@example.com",
    groups: ["admins", "ops"],
    is_admin: true,
  },
};

const meta = {
  title: "Pages/ProxiesPage",
  component: ProxiesPage,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          "Unified proxy workspace that follows the current config selector. Pick Global to manage the shared pool and allocations, or pick a profile to manage local imports and global-pool usage.",
      },
    },
  },
} satisfies Meta<typeof ProxiesPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const GlobalConfig: Story = {
  args: {
    mode: "global",
    profiles,
    currentUser,
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
  render: (args) => (
    <AppShell
      profileId={GLOBAL_PROFILE_ID}
      profiles={profiles}
      profilesLoading={false}
      profilesCreating={false}
      profilesError={null}
      healthStatus="ok"
      currentUser={currentUser}
      onProfileIdChange={() => undefined}
      onCreateProfile={async (value: string) => value}
      onRetryProfiles={() => undefined}
    >
      <ProxiesPage {...args} />
    </AppShell>
  ),
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(/import global proxy pool/i)).toBeVisible();
    await expect(await canvas.findByText(/global pool and profile allocations/i)).toBeVisible();
  },
};

export const ProfileConfig: Story = {
  args: {
    mode: "profile",
    profileId: "edge-jp",
    currentUser,
    profileLoadResponse: {
      loaded_proxies: 4,
      distinct_ips: 3,
      warnings: [],
    },
    profileLoadError: null,
    loadingProfile: false,
    proxySettings: {
      profile_id: "edge-jp",
      use_global_proxies: true,
    },
    proxySettingsLoading: false,
    proxySettingsError: null,
    updatingSettings: false,
    showProxyPolicy: true,
    onLoadProfile: fn(),
    onToggleUseGlobalProxies: fn(),
  },
  render: (args) => (
    <AppShell
      profileId="edge-jp"
      profiles={profiles}
      profilesLoading={false}
      profilesCreating={false}
      profilesError={null}
      healthStatus="ok"
      currentUser={currentUser}
      onProfileIdChange={() => undefined}
      onCreateProfile={async (value: string) => value}
      onRetryProfiles={() => undefined}
    >
      <ProxiesPage {...args} />
    </AppShell>
  ),
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(/import local proxy pool/i)).toBeVisible();
    await expect(await canvas.findByText(/use global pool for edge-jp/i)).toBeVisible();
  },
};

export const ZhCN: Story = {
  ...GlobalConfig,
  globals: {
    locale: "zh-CN",
  },
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(/导入全局代理池/i)).toBeVisible();
    await expect(await canvas.findByText(/全局池与配置分配/i)).toBeVisible();
  },
};

export const AccessDenied: Story = {
  args: {
    mode: "global",
    profiles,
    currentUser,
    accessDenied: true,
    authError: null,
    globalLoadResponse: null,
    globalLoadError: null,
    loadingGlobal: false,
    inventory: null,
    inventoryLoading: false,
    inventoryError: null,
    onLoadGlobal: fn(),
    onReassignNode: fn(),
    onDeleteNode: fn(),
  },
  render: (args) => (
    <AppShell
      profileId={GLOBAL_PROFILE_ID}
      profiles={profiles}
      profilesLoading={false}
      profilesCreating={false}
      profilesError={null}
      healthStatus="ok"
      currentUser={currentUser}
      onProfileIdChange={() => undefined}
      onCreateProfile={async (value: string) => value}
      onRetryProfiles={() => undefined}
    >
      <ProxiesPage {...args} />
    </AppShell>
  ),
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(/admin access required/i)).toBeVisible();
  },
};
