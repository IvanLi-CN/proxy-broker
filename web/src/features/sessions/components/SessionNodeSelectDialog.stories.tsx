import type { Meta, StoryObj } from "@storybook/react-vite";
import type { ComponentProps } from "react";
import { useState } from "react";
import { expect, fn, userEvent, within } from "storybook/test";

import { SessionNodeSelectDialog } from "@/features/sessions/components/SessionNodeSelectDialog";
import type { ProxyNodeLiveState } from "@/hooks/use-proxy-operation-events";
import type { SessionNodeOptionItem } from "@/lib/types";
import { sessionNodeOptionsFixture, sessionsFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Sessions/SessionNodeSelectDialog",
  component: SessionNodeSelectDialog,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Dialog for switching the current session to another node, with keyword filtering, two pinned recency groups, and region/source grouping.",
      },
    },
  },
  args: {
    open: true,
    session: sessionsFixture.sessions[0] ?? null,
    isPending: false,
    error: null,
    onOpenChange: fn(),
    onSearch: fn(async () => sessionNodeOptionsFixture.items),
    onSubmit: fn(),
    onProbeNodes: fn(),
  },
} satisfies Meta<typeof SessionNodeSelectDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

function isVisibleElement(element: HTMLElement) {
  const style = element.ownerDocument.defaultView?.getComputedStyle(element);
  return Boolean(element.getClientRects().length) && style?.display !== "none";
}

async function findVisibleButton(
  dialog: ReturnType<typeof within>,
  options: { name: string | RegExp },
) {
  const buttons = await dialog.findAllByRole("button", options);
  const visibleButton = buttons.find((button: HTMLElement) => isVisibleElement(button));
  if (!visibleButton) {
    throw new Error(`Expected a visible button matching ${String(options.name)}`);
  }
  return visibleButton;
}

function buildLongRegionNodeOptions(): SessionNodeOptionItem[] {
  const baseItems = sessionNodeOptionsFixture.items;
  return [
    ...baseItems,
    ...baseItems.map((item, index) => ({
      ...item,
      node_id: `${item.node_id}-long-region-${index}`,
      proxy_name: `${item.proxy_name}-LongRegion-${index + 1}`,
      country_code: "US",
      country_name: "United States",
      region_name: "California",
      city:
        index === 0
          ? "San Francisco International Financial District East"
          : "San Francisco International Financial District West",
      session_last_used_at:
        item.session_last_used_at == null ? null : item.session_last_used_at - 60,
      project_last_used_at:
        item.project_last_used_at == null ? null : item.project_last_used_at - 60,
    })),
  ];
}

function expectGroupItemsWithinBounds(ownerDocument: Document) {
  const groupButtons = Array.from(
    ownerDocument.querySelectorAll<HTMLElement>('[data-testid="session-node-group-option"]'),
  );
  const viewport = ownerDocument.querySelector<HTMLElement>(
    '[data-testid="session-node-group-scroll"] [data-slot="scroll-area-viewport"]',
  );
  if (!viewport) {
    throw new Error("Expected session node group viewport to exist");
  }
  expect(viewport.scrollWidth).toBeLessThanOrEqual(Math.ceil(viewport.clientWidth) + 1);
  expect(groupButtons.length).toBeGreaterThan(0);

  for (const button of groupButtons) {
    const buttonRect = button.getBoundingClientRect();
    const viewportRect = viewport.getBoundingClientRect();
    expect(buttonRect.left).toBeGreaterThanOrEqual(Math.floor(viewportRect.left) - 1);
    expect(buttonRect.right).toBeLessThanOrEqual(Math.ceil(viewportRect.right) + 1);
    expect(button.scrollWidth).toBeLessThanOrEqual(Math.ceil(button.clientWidth) + 1);
    for (const child of Array.from(button.children)) {
      const childRect = child.getBoundingClientRect();
      expect(childRect.left).toBeGreaterThanOrEqual(Math.floor(buttonRect.left) - 1);
      expect(childRect.right).toBeLessThanOrEqual(Math.ceil(buttonRect.right) + 1);
    }
  }
}

function ProbeCurrentNodeWithoutReloadRender(args: ComponentProps<typeof SessionNodeSelectDialog>) {
  const baseSession = sessionsFixture.sessions[0] ?? null;
  const [sessionVersion, setSessionVersion] = useState(0);
  const [liveNodeStates, setLiveNodeStates] = useState<Record<string, ProxyNodeLiveState>>({});
  const session = baseSession
    ? {
        ...baseSession,
        proxy_name:
          sessionVersion === 0
            ? baseSession.proxy_name
            : `${baseSession.proxy_name} refreshed ${sessionVersion}`,
      }
    : null;

  return (
    <SessionNodeSelectDialog
      {...args}
      session={session}
      liveNodeStates={liveNodeStates}
      onProbeNodes={async (nodeIds) => {
        const nodeId = nodeIds[0];
        if (!nodeId) {
          return;
        }
        setSessionVersion((current) => current + 1);
        setLiveNodeStates({
          [nodeId]: {
            nodeId,
            runId: "story-probe-live",
            kind: "proxy_latency_probe",
            latestSampleMs: 91,
            latestRound: 1,
            samplesTotal: 1,
            progressCurrent: 1,
            progressTotal: 3,
            message: "Probe sample finished.",
            at: Date.now(),
          },
        });
      }}
    />
  );
}

function ProbeListNodeWithoutReloadRender(args: ComponentProps<typeof SessionNodeSelectDialog>) {
  const baseSession = sessionsFixture.sessions[0] ?? null;
  const [sessionVersion, setSessionVersion] = useState(0);
  const [liveNodeStates, setLiveNodeStates] = useState<Record<string, ProxyNodeLiveState>>({});
  const session = baseSession
    ? {
        ...baseSession,
        proxy_name:
          sessionVersion === 0
            ? baseSession.proxy_name
            : `${baseSession.proxy_name} refreshed ${sessionVersion}`,
      }
    : null;

  return (
    <SessionNodeSelectDialog
      {...args}
      session={session}
      liveNodeStates={liveNodeStates}
      onProbeNodes={async (nodeIds) => {
        const nodeId = nodeIds[0];
        if (!nodeId) {
          return;
        }
        setSessionVersion((current) => current + 1);
        setLiveNodeStates({
          [nodeId]: {
            nodeId,
            runId: "story-list-probe-live",
            kind: "proxy_latency_probe",
            latestSampleMs: 91,
            latestRound: 1,
            samplesTotal: 1,
            progressCurrent: 1,
            progressTotal: 3,
            message: "Probe sample finished.",
            at: Date.now(),
          },
        });
      }}
    />
  );
}

export const Default: Story = {
  args: {},
  async play({ canvasElement }) {
    const dialog = within(canvasElement.ownerDocument.body);
    await expect(await dialog.findByText(/Group nodes/i)).toBeInTheDocument();
    await expect(
      await dialog.findByRole("button", { name: /Current session last used/i }),
    ).toBeInTheDocument();
    await expect((await dialog.findAllByRole("button", { name: /Japan/i })).length).toBeGreaterThan(
      0,
    );
    await expect((await dialog.findAllByText(/JP-Tokyo-Entry/i)).length).toBeGreaterThan(0);
    await expect((await dialog.findAllByText(/Japan \/ Tokyo \/ Chiyoda/i)).length).toBeGreaterThan(
      0,
    );
    const latencyLabels = await dialog.findAllByText(/88 ms/i);
    await expect(latencyLabels.length).toBeGreaterThan(0);
    await expect((await dialog.findAllByText(/Current session last used/i)).length).toBeGreaterThan(
      0,
    );
    await expect(
      await findVisibleButton(dialog, { name: /Probe current group/i }),
    ).toBeInTheDocument();
    await expect(
      await findVisibleButton(dialog, { name: /Probe current node JP-Tokyo-Entry/i }),
    ).toBeInTheDocument();
    await expect(
      await findVisibleButton(dialog, { name: /Probe node JP-Tokyo-Entry/i }),
    ).toBeInTheDocument();
    const firstLatencyLabel = latencyLabels[0];
    if (!firstLatencyLabel) {
      throw new Error("Expected an 88 ms latency label");
    }
    await userEvent.hover(firstLatencyLabel);
    await expect((await dialog.findAllByText(/140 ms/i)).length).toBeGreaterThan(0);
  },
};

export const Switching: Story = {
  args: {
    isPending: true,
  },
};

export const ZhCN: Story = {
  args: {},
  globals: {
    locale: "zh-CN",
  },
};

export const SourceGrouping: Story = {
  args: {},
  async play({ canvasElement }) {
    const dialog = within(canvasElement.ownerDocument.body);
    await userEvent.click(await dialog.findByRole("radio", { name: /Group by subscription/i }));
    await expect(
      (await dialog.findAllByRole("button", { name: /browser-core/i })).length,
    ).toBeGreaterThan(0);
    const fallbackButtons = await dialog.findAllByRole("button", { name: /fallback-lab/i });
    await userEvent.click(fallbackButtons[0] as HTMLElement);
    await expect(
      await dialog.findByRole("button", { name: /Select node US-SanJose-Edge/i }),
    ).toBeInTheDocument();
    await userEvent.click(await findVisibleButton(dialog, { name: /Probe current group/i }));
  },
};

export const ProbingQueued: Story = {
  args: {
    probingNodeIds: ["node-jp-tokyo-entry", "node-jp-osaka-edge", "node-us-sanjose-edge"],
  },
  async play({ canvasElement }) {
    const dialog = within(canvasElement.ownerDocument.body);
    await expect(await findVisibleButton(dialog, { name: /Probe current group/i })).toBeDisabled();
    await expect(
      await findVisibleButton(dialog, { name: /Probe current node JP-Tokyo-Entry/i }),
    ).toBeDisabled();
    await expect(
      await findVisibleButton(dialog, { name: /Probe node JP-Tokyo-Entry/i }),
    ).toBeDisabled();
  },
};

export const GroupListWidthConstrained: Story = {
  args: {
    onSearch: fn(async () => buildLongRegionNodeOptions()),
  },
  async play({ canvasElement }) {
    const dialog = within(canvasElement.ownerDocument.body);
    await expect(
      (
        await dialog.findAllByRole("button", {
          name: /United States \/ California \/ San Francisco International Financial District/i,
        })
      ).length,
    ).toBeGreaterThan(0);
    expectGroupItemsWithinBounds(canvasElement.ownerDocument);
  },
};

export const ProbeCurrentNodeWithoutReload: Story = {
  render: ProbeCurrentNodeWithoutReloadRender,
  async play({ canvasElement }) {
    const dialog = within(canvasElement.ownerDocument.body);
    await expect(
      await dialog.findByRole("button", { name: /Select node US-SanJose-Edge/i }),
    ).toBeInTheDocument();
    await userEvent.click(await findVisibleButton(dialog, { name: /Probe current node/i }));
    await expect(dialog.queryByText(/Loading node options/i)).not.toBeInTheDocument();
    await expect(
      await dialog.findByRole("button", { name: /Select node US-SanJose-Edge/i }),
    ).toBeInTheDocument();
    await expect((await dialog.findAllByText(/91 ms/i)).length).toBeGreaterThan(0);
  },
};

export const ProbeListNodeWithoutReload: Story = {
  render: ProbeListNodeWithoutReloadRender,
  async play({ canvasElement }) {
    const dialog = within(canvasElement.ownerDocument.body);
    await expect(
      await dialog.findByRole("button", { name: /Select node US-SanJose-Edge/i }),
    ).toBeInTheDocument();
    await userEvent.click(await findVisibleButton(dialog, { name: /Probe node US-SanJose-Edge/i }));
    await expect(dialog.queryByText(/Loading node options/i)).not.toBeInTheDocument();
    await expect(
      await dialog.findByRole("button", { name: /Select node US-SanJose-Edge/i }),
    ).toBeInTheDocument();
    await expect((await dialog.findAllByText(/91 ms/i)).length).toBeGreaterThan(0);
  },
};
