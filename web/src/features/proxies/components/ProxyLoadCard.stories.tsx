import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { ProxyLoadCard } from "@/features/proxies/components/ProxyLoadCard";

const meta = {
  title: "Features/Proxies/ProxyLoadCard",
  component: ProxyLoadCard,
  tags: ["autodocs"],
  args: {
    eyebrow: "Current project",
    title: "Import local pool for edge-jp",
    description:
      "Import nodes for the current project only. These nodes stay local unless you later reassign them from the global inventory.",
    scopeChip: "allocation defaults to edge-jp",
    pending: false,
    response: {
      loaded_proxies: 6,
      distinct_ips: 4,
      resolved_name: "edge-feed",
      resolved_name_source: "parsed_source",
      subscription_metadata: {
        source_title: "edge-feed",
        total_bytes: 100 * 1024 ** 3,
        remaining_bytes: 70 * 1024 ** 3,
        used_bytes: 30 * 1024 ** 3,
        expire_at: 1_741_748_800,
      },
      warnings: [],
    },
    error: null,
    defaultValue: "https://example.com/project-subscription.yaml",
    submitLabel: "Import project pool",
    successTitle: "Project pool updated",
    successDescription: "Imported 6 proxies across 4 distinct IPs into project edge-jp.",
    onSubmit: fn(),
  },
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component:
          "Compact subscription import card used by both the global proxies workspace and project-scoped local import surfaces.",
      },
    },
  },
} satisfies Meta<typeof ProxyLoadCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ErrorState: Story = {
  args: {
    response: null,
    error: "subscription_fetch_failed: upstream not reachable",
  },
};

export const ManualNamePreferred: Story = {
  args: {
    response: {
      loaded_proxies: 6,
      distinct_ips: 4,
      resolved_name: "ops-feed",
      resolved_name_source: "existing_import",
      subscription_metadata: {
        source_title: "Tokyo Premium Feed",
        total_bytes: 100 * 1024 ** 3,
        remaining_bytes: 70 * 1024 ** 3,
        expire_at: 1_741_748_800,
      },
      warnings: [],
    },
  },
};

export const FilterWarning: Story = {
  args: {
    response: {
      loaded_proxies: 5,
      distinct_ips: 4,
      resolved_name: "edge-feed",
      resolved_name_source: "parsed_source",
      subscription_metadata: {
        source_title: "edge-feed",
        total_bytes: 100 * 1024 ** 3,
        remaining_bytes: 68 * 1024 ** 3,
      },
      warnings: ["filtered informational subscription entry `剩余流量 68GB`"],
    },
  },
};

export const NodeGroupMode: Story = {
  args: {
    response: null,
    error: null,
    onSubmit: fn(),
  },
  async play({ canvasElement }) {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("tab", { name: /nodes/i }));
    await expect(await canvas.findByLabelText(/nodes content/i)).toBeVisible();
  },
};

export const Interaction: Story = {
  args: {
    response: null,
    error: null,
    onSubmit: fn(),
  },
  async play({ canvasElement, args }) {
    const canvas = within(canvasElement);
    await userEvent.clear(canvas.getByLabelText("Name"));
    await userEvent.clear(canvas.getByLabelText("Value"));
    await userEvent.type(canvas.getByLabelText("Value"), "https://example.com/feed.yaml");
    await userEvent.click(canvas.getByRole("button", { name: /import project pool/i }));
    expect(args.onSubmit).toHaveBeenCalledWith({
      source: {
        type: "url",
        value: "https://example.com/feed.yaml",
      },
    });
  },
};
