import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { SessionNodeSelectDialog } from "@/features/sessions/components/SessionNodeSelectDialog";
import { I18nProvider } from "@/i18n";
import { sessionIpNodeOptionsFixture, sessionsFixture } from "@/mocks/fixtures";

function renderWithProviders(node: ReactNode) {
  return render(
    <I18nProvider initialLocale="en-US">
      <TooltipProvider>{node}</TooltipProvider>
    </I18nProvider>,
  );
}

describe("SessionNodeSelectDialog", () => {
  it("loads IP options and submits the selected IP candidate set", async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn().mockResolvedValue(sessionIpNodeOptionsFixture.groups);
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
      />,
    );

    await waitFor(() => {
      expect(onSearch).toHaveBeenCalledWith({
        query: undefined,
        group_by: "subscription",
        session_id: sessionsFixture.sessions[0]?.session_id,
        limit: 80,
      });
    });

    await user.click(screen.getByRole("button", { name: /198\.51\.100\.42/i }));
    await user.click(screen.getByRole("button", { name: /Use selected candidates/i }));

    expect(onSubmit).toHaveBeenCalledWith(sessionsFixture.sessions[0]?.session_id, {
      selected_ip: "198.51.100.42",
      candidate_node_ids: ["node-us-sanjose-edge"],
    });
  });

  it("preserves in-progress selections when the same session refetches", async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn().mockResolvedValue(sessionIpNodeOptionsFixture.groups);
    const onSubmit = vi.fn();
    const session = sessionsFixture.sessions[0] ?? null;
    const { rerender } = renderWithProviders(
      <SessionNodeSelectDialog
        open
        session={session}
        isPending={false}
        error={null}
        onOpenChange={vi.fn()}
        onSearch={onSearch}
        onSubmit={onSubmit}
      />,
    );

    await waitFor(() => {
      expect(onSearch).toHaveBeenCalled();
    });

    await user.click(screen.getByRole("button", { name: /198\.51\.100\.42/i }));

    rerender(
      <I18nProvider initialLocale="en-US">
        <TooltipProvider>
          <SessionNodeSelectDialog
            open
            session={session ? { ...session, proxy_name: "JP-Tokyo-Entry (refetched)" } : null}
            isPending={false}
            error={null}
            onOpenChange={vi.fn()}
            onSearch={onSearch}
            onSubmit={onSubmit}
          />
        </TooltipProvider>
      </I18nProvider>,
    );

    await user.click(screen.getByRole("button", { name: /Use selected candidates/i }));

    expect(onSubmit).toHaveBeenCalledWith(session?.session_id, {
      selected_ip: "198.51.100.42",
      candidate_node_ids: ["node-us-sanjose-edge"],
    });
  });

  it("reinitializes selections when switching to another session while open", async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn().mockResolvedValue(sessionIpNodeOptionsFixture.groups);
    const onSubmit = vi.fn();
    const firstSession = sessionsFixture.sessions[0] ?? null;
    const secondSession = sessionsFixture.sessions[1] ?? null;
    const { rerender } = renderWithProviders(
      <SessionNodeSelectDialog
        open
        session={firstSession}
        isPending={false}
        error={null}
        onOpenChange={vi.fn()}
        onSearch={onSearch}
        onSubmit={onSubmit}
      />,
    );

    await waitFor(() => {
      expect(onSearch).toHaveBeenCalled();
    });
    await user.click(screen.getByRole("button", { name: /198\.51\.100\.42/i }));

    rerender(
      <I18nProvider initialLocale="en-US">
        <TooltipProvider>
          <SessionNodeSelectDialog
            open
            session={secondSession}
            isPending={false}
            error={null}
            onOpenChange={vi.fn()}
            onSearch={onSearch}
            onSubmit={onSubmit}
          />
        </TooltipProvider>
      </I18nProvider>,
    );

    await user.click(screen.getByRole("button", { name: /Use selected candidates/i }));

    expect(onSubmit).toHaveBeenCalledWith(secondSession?.session_id, {
      selected_ip: "203.0.113.88",
      candidate_node_ids: ["node-jp-osaka-edge"],
    });
  });

  it("disables submit and unchecks the IP when all candidate nodes are cleared", async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn().mockResolvedValue(sessionIpNodeOptionsFixture.groups);
    renderWithProviders(
      <SessionNodeSelectDialog
        open
        session={sessionsFixture.sessions[0] ?? null}
        isPending={false}
        error={null}
        onOpenChange={vi.fn()}
        onSearch={onSearch}
        onSubmit={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(onSearch).toHaveBeenCalled();
    });

    const submit = screen.getByRole("button", { name: /Use selected candidates/i });
    expect(submit).toBeEnabled();

    await user.click(await screen.findByRole("button", { name: /198\.51\.100\.42/i }));
    await user.click(screen.getByRole("button", { name: /Clear nodes/i }));

    expect(submit).toBeDisabled();
    expect(screen.getByText("0 candidate nodes selected")).toBeInTheDocument();
  });
});
