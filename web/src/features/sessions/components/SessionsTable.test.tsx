import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import { SessionsTable } from "@/features/sessions/components/SessionsTable";
import { I18nProvider } from "@/i18n";
import { sessionsFixture } from "@/mocks/fixtures";

function renderWithProviders(node: ReactNode) {
  return render(<I18nProvider initialLocale="en-US">{node}</I18nProvider>);
}

describe("SessionsTable", () => {
  it("shows selected IP geography and removes the redundant port badge", () => {
    renderWithProviders(
      <SessionsTable
        sessions={sessionsFixture.sessions}
        isLoading={false}
        closingSessionId={null}
        switchingSessionId={null}
        onEditSession={vi.fn()}
        onCloseSession={vi.fn()}
      />,
    );

    expect(screen.getByText("Japan / Tokyo / Chiyoda")).toBeInTheDocument();
    expect(screen.getByText("Japan / Osaka")).toBeInTheDocument();
    expect(screen.queryByText(/port 10080/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/port 10081/i)).not.toBeInTheDocument();
  });
});
