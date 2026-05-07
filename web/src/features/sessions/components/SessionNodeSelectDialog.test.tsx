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
    expect(screen.getAllByRole("button", { name: /Japan/i }).length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: /Current profile last used/i }));
    await user.click(screen.getByRole("radio", { name: /Group by subscription/i }));
    expect(screen.getAllByRole("button", { name: /fallback-lab/i }).length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: /US-SanJose-Edge/i }));
    await user.click(screen.getByRole("button", { name: /Use selected node/i }));

    expect(onSubmit).toHaveBeenCalledWith(sessionsFixture.sessions[0]?.session_id, {
      node_id: "node-us-sanjose-edge",
    });
  });
});
