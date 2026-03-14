import type { Meta, StoryObj } from "@storybook/react-vite";

import { RouteHero } from "@/components/RouteHero";
import { Button } from "@/components/ui/button";
import { WorkflowRail } from "@/components/WorkflowRail";

const meta = {
  title: "Components/RouteHero",
  component: RouteHero,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          "Large route hero used to anchor each operator workspace with control-room hierarchy, state badges, and an auxiliary rail.",
      },
    },
  },
  args: {
    eyebrow: "Control room",
    title: "Operate the pool with a clearer runway and tighter feedback.",
    description:
      "Use the hero to establish route context, live state, and the next operator action without burning vertical space on repetitive prose.",
    badges: [
      { label: "service healthy", tone: "positive" },
      { label: "2 active sessions", tone: "neutral" },
      { label: "warnings queued", tone: "warning" },
    ],
    actions: <Button>Primary action</Button>,
    aside: (
      <WorkflowRail
        eyebrow="Run order"
        title="Keep the route tidy"
        steps={[
          { title: "Load", description: "Refresh the pool before sampling it." },
          { title: "Inspect", description: "Read the resulting state panels and tables." },
          { title: "Act", description: "Open or close listeners only after the data looks sane." },
        ]}
      />
    ),
  },
} satisfies Meta<typeof RouteHero>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
