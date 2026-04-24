import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import { SessionNodeSelectDialog } from "@/features/sessions/components/SessionNodeSelectDialog";
import { I18nProvider } from "@/i18n";
import { sessionNodeOptionsFixture, sessionsFixture } from "@/mocks/fixtures";

function renderWithProviders(node: ReactNode) {
  return render(<I18nProvider initialLocale="en-US">{node}</I18nProvider>);
}

describe("SessionNodeSelectDialog", () => {
  it("loads node options, switches sort mode, and submits the selected node", async () => {
    const user = userEvent.setup();
    const onSearch = vi
      .fn()
      .mockResolvedValueOnce(sessionNodeOptionsFixture.items)
      .mockResolvedValueOnce([...sessionNodeOptionsFixture.items].reverse());
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
        limit: 50,
      });
    });

    await user.click(screen.getByRole("combobox", { name: /Sort by/i }));
    await user.click(screen.getByRole("option", { name: /Current profile last used/i }));

    await waitFor(() => {
      expect(onSearch).toHaveBeenLastCalledWith(sessionsFixture.sessions[0]?.session_id, {
        query: undefined,
        sort_mode: "profile_recent",
        limit: 50,
      });
    });

    await user.click(screen.getByRole("button", { name: /US-SanJose-Edge/i }));
    await user.click(screen.getByRole("button", { name: /Use selected node/i }));

    expect(onSubmit).toHaveBeenCalledWith(sessionsFixture.sessions[0]?.session_id, {
      node_id: "node-us-sanjose-edge",
    });
  });
});
