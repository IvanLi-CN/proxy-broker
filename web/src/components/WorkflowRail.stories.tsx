import type { Meta, StoryObj } from "@storybook/react-vite";

import { WorkflowRail } from "@/components/WorkflowRail";

const meta = {
  title: "Components/WorkflowRail",
  component: WorkflowRail,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Compact step rail for operator runbooks, page-side instructions, and concise workflow sequencing.",
      },
    },
  },
  args: {
    eyebrow: "Runbook",
    title: "Three-step operator flow",
    steps: [
      { title: "Load a fresh feed", description: "Bring in the newest upstream inventory." },
      { title: "Probe and filter", description: "Validate geo and latency before extracting." },
      {
        title: "Open only what you need",
        description: "Keep the live listener list small and intentional.",
      },
    ],
  },
} satisfies Meta<typeof WorkflowRail>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
