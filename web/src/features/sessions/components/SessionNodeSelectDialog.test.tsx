import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import { SessionNodeSelectDialog } from "@/features/sessions/components/SessionNodeSelectDialog";
import { I18nProvider } from "@/i18n";
import { sessionIpNodeOptionsFixture, sessionsFixture } from "@/mocks/fixtures";

function renderWithProviders(node: ReactNode) {
  return render(<I18nProvider initialLocale="en-US">{node}</I18nProvider>);
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
});
