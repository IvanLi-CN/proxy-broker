import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { SessionNodeSelectDialog } from "@/features/sessions/components/SessionNodeSelectDialog";
import { I18nProvider } from "@/i18n";
import { sessionNodeOptionsFixture, sessionsFixture } from "@/mocks/fixtures";

function renderWithProviders(node: ReactNode) {
  return render(
    <I18nProvider initialLocale="en-US">
      <TooltipProvider>{node}</TooltipProvider>
    </I18nProvider>,
  );
}

describe("SessionNodeSelectDialog", () => {
  it("loads all node options, switches groups, and submits the selected node", async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn().mockResolvedValue(sessionNodeOptionsFixture.items);
    const onSubmit = vi.fn();

    renderWithProviders(
      <SessionNodeSelectDialog
        open
        session={sessionsFixture.sessions[0] ?? null}
        isPending={false}
        error={null}
        onOpenChange={vi.fn()}
        onSearch={onSearch}
        onSubmit={onSubmit}
        onProbeNodes={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(onSearch).toHaveBeenCalledWith(sessionsFixture.sessions[0]?.session_id, {
        query: undefined,
        sort_mode: "session_recent",
      });
    });

    expect(screen.getByRole("button", { name: /Current session last used/i })).toHaveClass(
      "bg-primary",
    );
    expect(screen.getAllByText("Japan / Tokyo / Chiyoda").length).toBeGreaterThan(0);
    expect(screen.getAllByRole("button", { name: /Japan/i }).length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: /Current project last used/i }));
    await user.click(screen.getByRole("radio", { name: /Group by subscription/i }));
    expect(screen.getAllByRole("button", { name: /fallback-lab/i }).length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: /Select node US-SanJose-Edge/i }));
    await user.click(screen.getByRole("button", { name: /Use selected node/i }));

    expect(onSubmit).toHaveBeenCalledWith(sessionsFixture.sessions[0]?.session_id, {
      node_id: "node-us-sanjose-edge",
    });
  }, 25_000);

  it("probes the current visible group and single nodes without changing selection", async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn().mockResolvedValue(sessionNodeOptionsFixture.items);
    const onSubmit = vi.fn();
    const onProbeNodes = vi.fn();

    renderWithProviders(
      <SessionNodeSelectDialog
        open
        session={sessionsFixture.sessions[0] ?? null}
        isPending={false}
        error={null}
        onOpenChange={vi.fn()}
        onSearch={onSearch}
        onSubmit={onSubmit}
        onProbeNodes={onProbeNodes}
      />,
    );

    await waitFor(() => {
      expect(onSearch).toHaveBeenCalled();
    });

    await user.click(screen.getByRole("button", { name: /Probe current node JP-Tokyo-Entry/i }));
    expect(onProbeNodes).toHaveBeenLastCalledWith(["node-jp-tokyo-entry"]);

    await user.click(screen.getByRole("button", { name: /Probe current group/i }));
    expect(onProbeNodes).toHaveBeenLastCalledWith([
      "node-jp-tokyo-entry",
      "node-jp-osaka-edge",
      "node-us-sanjose-edge",
    ]);

    await user.click(screen.getByRole("radio", { name: /Group by subscription/i }));
    const fallbackButtons = screen.getAllByRole("button", { name: /fallback-lab/i });
    await user.click(fallbackButtons[0] as HTMLElement);
    await user.click(screen.getByRole("button", { name: /Probe current group/i }));
    expect(onProbeNodes).toHaveBeenLastCalledWith(["node-us-sanjose-edge"]);

    await user.click(screen.getByRole("button", { name: /Probe node US-SanJose-Edge/i }));
    expect(onProbeNodes).toHaveBeenLastCalledWith(["node-us-sanjose-edge"]);

    await user.click(screen.getByRole("button", { name: /Use selected node/i }));
    expect(onSubmit).not.toHaveBeenCalled();
  }, 25_000);

  it("keeps an in-progress node selection when session polling refreshes the same session", async () => {
    const user = userEvent.setup();
    const session = sessionsFixture.sessions[0];
    const onSearch = vi.fn().mockResolvedValue(sessionNodeOptionsFixture.items);
    const onSubmit = vi.fn();
    if (!session) {
      throw new Error("Expected session fixture.");
    }

    const renderDialog = (nextSession: typeof session) => (
      <I18nProvider initialLocale="en-US">
        <TooltipProvider>
          <SessionNodeSelectDialog
            open
            session={nextSession}
            isPending={false}
            error={null}
            onOpenChange={vi.fn()}
            onSearch={onSearch}
            onSubmit={onSubmit}
            onProbeNodes={vi.fn()}
          />
        </TooltipProvider>
      </I18nProvider>
    );

    const { rerender } = render(renderDialog(session));

    await waitFor(() => {
      expect(onSearch).toHaveBeenCalled();
    });

    await user.click(screen.getByRole("button", { name: /Select node US-SanJose-Edge/i }));
    rerender(renderDialog({ ...session, proxy_name: `${session.proxy_name} refreshed` }));
    await user.click(screen.getByRole("button", { name: /Use selected node/i }));

    expect(onSubmit).toHaveBeenCalledWith(session.session_id, {
      node_id: "node-us-sanjose-edge",
    });
  });

  it("keeps the latest completed probe result visible without disabling another probe", async () => {
    const onSearch = vi.fn().mockResolvedValue(sessionNodeOptionsFixture.items);

    renderWithProviders(
      <SessionNodeSelectDialog
        open
        session={sessionsFixture.sessions[0] ?? null}
        isPending={false}
        error={null}
        onOpenChange={vi.fn()}
        onSearch={onSearch}
        onSubmit={vi.fn()}
        onProbeNodes={vi.fn()}
        probeNodeStates={{
          "node-us-sanjose-edge": {
            nodeId: "node-us-sanjose-edge",
            runId: "run-completed",
            kind: "proxy_latency_probe",
            latestSampleMs: 88,
            latestRound: 5,
            samplesTotal: 5,
            progressCurrent: 3,
            progressTotal: 3,
            message: "probe complete",
            at: Date.now(),
          },
        }}
      />,
    );

    await waitFor(() => {
      expect(screen.getAllByText("88 ms").length).toBeGreaterThan(0);
    });

    expect(screen.getByRole("button", { name: /Probe node US-SanJose-Edge/i })).toBeEnabled();
  });

  it("keeps queued probe actions disabled from pending node ids", async () => {
    const onSearch = vi.fn().mockResolvedValue(sessionNodeOptionsFixture.items);

    renderWithProviders(
      <SessionNodeSelectDialog
        open
        session={sessionsFixture.sessions[0] ?? null}
        isPending={false}
        error={null}
        onOpenChange={vi.fn()}
        onSearch={onSearch}
        onSubmit={vi.fn()}
        onProbeNodes={vi.fn()}
        probingNodeIds={["node-jp-tokyo-entry"]}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Probe current group/i })).toBeDisabled();
    });

    expect(
      screen.getByRole("button", { name: /Probe current node JP-Tokyo-Entry/i }),
    ).toBeDisabled();
    expect(screen.getByRole("button", { name: /Probe node JP-Tokyo-Entry/i })).toBeDisabled();
  });

  it("keeps group probe disabled while a visible node has a live probe state", async () => {
    const onSearch = vi.fn().mockResolvedValue(sessionNodeOptionsFixture.items);

    renderWithProviders(
      <SessionNodeSelectDialog
        open
        session={sessionsFixture.sessions[0] ?? null}
        isPending={false}
        error={null}
        onOpenChange={vi.fn()}
        onSearch={onSearch}
        onSubmit={vi.fn()}
        onProbeNodes={vi.fn()}
        liveNodeStates={{
          "node-jp-tokyo-entry": {
            nodeId: "node-jp-tokyo-entry",
            runId: "run-live",
            kind: "proxy_latency_probe",
            latestSampleMs: 91,
            progressCurrent: 1,
            progressTotal: 15,
            message: "Probe sample finished.",
            at: Date.now(),
          },
        }}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Probe current group/i })).toBeDisabled();
    });
  });
});
