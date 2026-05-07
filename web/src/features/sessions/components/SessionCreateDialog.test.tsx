import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { SessionCreateDialog } from "@/features/sessions/components/SessionCreateDialog";
import { I18nProvider } from "@/i18n";
import { sessionIpNodeOptionsFixture } from "@/mocks/fixtures";

function renderWithProviders(node: ReactNode) {
  return render(
    <I18nProvider initialLocale="en-US">
      <TooltipProvider>{node}</TooltipProvider>
    </I18nProvider>,
  );
}

describe("SessionCreateDialog", () => {
  it("requires an explicit IP selection before creating a session", async () => {
    const user = userEvent.setup();
    const searchIpNodeOptions = vi.fn().mockResolvedValue(sessionIpNodeOptionsFixture.groups);
    const onOpenSession = vi.fn();

    renderWithProviders(
      <SessionCreateDialog
        open
        onOpenChange={vi.fn()}
        opening={false}
        batchOpening={false}
        suggestedPort={20_001}
        onOpenSession={onOpenSession}
        onOpenBatch={vi.fn()}
        searchIpNodeOptions={searchIpNodeOptions}
      />,
    );

    await waitFor(() => {
      expect(searchIpNodeOptions).toHaveBeenCalledWith({
        query: undefined,
        group_by: "subscription",
        session_id: undefined,
        limit: 80,
      });
    });

    const submit = screen.getByRole("button", { name: /^Create session$/i });
    expect(screen.getByText("0 IPs selected")).toBeInTheDocument();
    expect(submit).toBeDisabled();

    await user.click(await screen.findByRole("button", { name: /203\.0\.113\.10/i }));
    expect(screen.getByText("1 IPs selected")).toBeInTheDocument();
    expect(submit).toBeEnabled();

    await user.click(submit);
    expect(onOpenSession).toHaveBeenCalledWith({
      selected_ip: "203.0.113.10",
      candidate_node_ids: ["node-jp-tokyo-entry", "node-jp-tokyo-backup"],
      desired_port: 20_001,
    });
  });

  it("clears local selections when reopened", async () => {
    const user = userEvent.setup();
    const searchIpNodeOptions = vi.fn().mockResolvedValue(sessionIpNodeOptionsFixture.groups);
    const props = {
      onOpenChange: vi.fn(),
      opening: false,
      batchOpening: false,
      suggestedPort: 20_001,
      onOpenSession: vi.fn(),
      onOpenBatch: vi.fn(),
      searchIpNodeOptions,
    };
    const { rerender } = renderWithProviders(<SessionCreateDialog open {...props} />);

    await waitFor(() => {
      expect(searchIpNodeOptions).toHaveBeenCalledWith({
        query: undefined,
        group_by: "subscription",
        session_id: undefined,
        limit: 80,
      });
    });

    await user.click(await screen.findByRole("button", { name: /203\.0\.113\.10/i }));
    expect(screen.getByText("1 IPs selected")).toBeInTheDocument();

    rerender(
      <I18nProvider initialLocale="en-US">
        <TooltipProvider>
          <SessionCreateDialog open={false} {...props} />
        </TooltipProvider>
      </I18nProvider>,
    );
    rerender(
      <I18nProvider initialLocale="en-US">
        <TooltipProvider>
          <SessionCreateDialog open {...props} />
        </TooltipProvider>
      </I18nProvider>,
    );

    await waitFor(() => {
      expect(searchIpNodeOptions).toHaveBeenCalledTimes(2);
    });
    await waitFor(() => {
      expect(screen.getByText("0 IPs selected")).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: /^Create session$/i })).toBeDisabled();
  });
});
