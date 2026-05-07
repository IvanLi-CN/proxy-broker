import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

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
          "Primary application chrome with a compact sidebar brand strip, project switcher, navigation, and top status rail.",
      },
    },
  },
  render: (args) => (
    <AppShell {...args}>
      <div className="rounded-3xl border border-border/70 bg-card/90 p-8 text-sm text-muted-foreground">
        路由内容会渲染在这里。
      </div>
    </AppShell>
  ),
  args: {
    projectId: "default",
    projects: ["default", "edge-jp", "lab-us"],
    projectsLoading: false,
    projectsCreating: false,
    projectsError: null,
    healthStatus: "ok",
    currentUser: {
      status: "resolved",
      identity: {
        authenticated: true,
        principal_type: "human",
        subject: "admin@example.com",
        email: "admin@example.com",
        groups: ["proxy-broker-admins"],
        is_admin: true,
      },
    },
    onProjectIdChange: () => undefined,
    onCreateProject: async (value: string) => value,
    onRetryProjects: () => undefined,
  },
} satisfies Meta<typeof AppShell>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};

export const ZhCN: Story = {
  args: {},
  globals: {
    locale: "zh-CN",
  },
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    const links = [
      canvas.getByRole("link", { name: /总览/i }),
      canvas.getByRole("link", { name: /任务/i }),
      canvas.getByRole("link", { name: /代理/i }),
      canvas.getByRole("link", { name: /IP 提取/i }),
      canvas.getByRole("link", { name: /会话/i }),
    ];

    for (const link of links) {
      await expect(link).toBeVisible();
    }

    const rects = links.map((link) => link.getBoundingClientRect());
    for (let index = 1; index < rects.length; index += 1) {
      expect(rects[index].top - rects[index - 1].bottom).toBeGreaterThan(0);
    }
  },
};

export const Anonymous: Story = {
  args: {
    currentUser: {
      status: "anonymous",
    },
  },
};
