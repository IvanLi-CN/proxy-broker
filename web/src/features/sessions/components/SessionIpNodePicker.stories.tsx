import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { SessionIpNodePicker } from "@/features/sessions/components/SessionIpNodePicker";
import type { SessionIpNodeOptionGroupItem } from "@/lib/types";
import { sessionIpNodeOptionsFixture } from "@/mocks/fixtures";

const latencyGradeGroups: SessionIpNodeOptionGroupItem[] = [
  {
    key: "latency-lab",
    label: "latency-lab",
    items: [
      {
        ip: "203.0.113.10",
        group_key: "latency-lab",
        group_label: "latency-lab",
        subscription_name: "latency-lab",
        country_code: "JP",
        country_name: "Japan",
        region_name: "Tokyo",
        city: "Chiyoda",
        last_used_at: 1_741_748_520,
        best_latency_ms: 88,
        nodes: [
          {
            node_id: "node-grade-excellent",
            proxy_name: "Grade-Excellent",
            import_name: "latency-lab",
            source_label: "latency-lab",
            country_code: "JP",
            country_name: "Japan",
            region_name: "Tokyo",
            city: "Chiyoda",
            last_probe_ok: true,
            median_latency_ms: 88,
            recent_probe_samples: [
              {
                node_id: "node-grade-excellent",
                ip: "203.0.113.10",
                target_url: "https://www.gstatic.com/generate_204",
                ok: true,
                latency_ms: 88,
                sampled_at: 1_741_748_540,
              },
            ],
            session_last_used_at: 1_741_748_520,
            project_last_used_at: 1_741_748_520,
          },
          {
            node_id: "node-grade-good",
            proxy_name: "Grade-Good",
            import_name: "latency-lab",
            source_label: "latency-lab",
            country_code: "JP",
            country_name: "Japan",
            region_name: "Tokyo",
            city: "Chiyoda",
            last_probe_ok: true,
            median_latency_ms: 180,
            recent_probe_samples: [
              {
                node_id: "node-grade-good",
                ip: "203.0.113.10",
                target_url: "https://www.gstatic.com/generate_204",
                ok: true,
                latency_ms: 180,
                sampled_at: 1_741_748_520,
              },
            ],
            session_last_used_at: 1_741_748_300,
            project_last_used_at: 1_741_748_300,
          },
          {
            node_id: "node-grade-fair",
            proxy_name: "Grade-Fair",
            import_name: "latency-lab",
            source_label: "latency-lab",
            country_code: "SG",
            country_name: "Singapore",
            region_name: null,
            city: "Singapore",
            last_probe_ok: true,
            median_latency_ms: 650,
            recent_probe_samples: [
              {
                node_id: "node-grade-fair",
                ip: "203.0.113.10",
                target_url: "https://www.gstatic.com/generate_204",
                ok: true,
                latency_ms: 650,
                sampled_at: 1_741_748_500,
              },
            ],
            session_last_used_at: 1_741_748_100,
            project_last_used_at: 1_741_748_100,
          },
          {
            node_id: "node-grade-poor",
            proxy_name: "Grade-Poor",
            import_name: "latency-lab",
            source_label: "latency-lab",
            country_code: "US",
            country_name: "United States",
            region_name: "California",
            city: "San Jose",
            last_probe_ok: true,
            median_latency_ms: 1250,
            recent_probe_samples: [
              {
                node_id: "node-grade-poor",
                ip: "203.0.113.10",
                target_url: "https://www.gstatic.com/generate_204",
                ok: true,
                latency_ms: 1250,
                sampled_at: 1_741_748_480,
              },
            ],
            session_last_used_at: 1_741_747_900,
            project_last_used_at: 1_741_747_900,
          },
        ],
      },
      {
        ip: "198.51.100.42",
        group_key: "latency-lab",
        group_label: "latency-lab",
        subscription_name: "latency-lab",
        country_code: "US",
        country_name: "United States",
        region_name: "California",
        city: "San Jose",
        last_used_at: null,
        best_latency_ms: null,
        nodes: [
          {
            node_id: "node-grade-failed",
            proxy_name: "Grade-Failed",
            import_name: "latency-lab",
            source_label: "latency-lab",
            country_code: "US",
            country_name: "United States",
            region_name: "California",
            city: "San Jose",
            last_probe_ok: false,
            median_latency_ms: null,
            recent_probe_samples: [
              {
                node_id: "node-grade-failed",
                ip: "198.51.100.42",
                target_url: "https://www.gstatic.com/generate_204",
                ok: false,
                latency_ms: null,
                sampled_at: 1_741_748_460,
              },
            ],
            session_last_used_at: null,
            project_last_used_at: 1_741_747_700,
          },
        ],
      },
    ],
  },
];

const meta = {
  title: "Features/Sessions/SessionIpNodePicker",
  component: SessionIpNodePicker,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Shared two-column IP to candidate-node picker for session creation and session proxy switching.",
      },
    },
  },
  args: {
    mode: "multiple",
    disabled: false,
    initialSelectedIp: "203.0.113.10",
    initialCandidateNodeIds: ["node-jp-tokyo-entry", "node-jp-tokyo-backup"],
    onSelectionChange: fn(),
    onSearch: fn(async () => sessionIpNodeOptionsFixture.groups),
  },
} satisfies Meta<typeof SessionIpNodePicker>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};

export const SingleSwitch: Story = {
  args: {
    mode: "single",
    sessionId: "sess-A7c2Kp9LmQ4RsT1v",
  },
};

export const Empty: Story = {
  args: {
    initialSelectedIp: null,
    initialCandidateNodeIds: [],
    onSearch: fn(async () => []),
  },
};

export const CompactViewport: Story = {
  args: {},
  parameters: {
    viewport: {
      defaultViewport: "mobile2",
    },
  },
};

export const LatencyGrades: Story = {
  args: {
    initialSelectedIp: "203.0.113.10",
    initialCandidateNodeIds: [
      "node-grade-excellent",
      "node-grade-good",
      "node-grade-fair",
      "node-grade-poor",
    ],
    onSearch: fn(async () => latencyGradeGroups),
  },
};

export const ZhCN: Story = {
  args: {},
  globals: {
    locale: "zh-CN",
  },
};
