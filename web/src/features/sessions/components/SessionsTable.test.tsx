import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SessionsTable } from "@/features/sessions/components/SessionsTable";
import { I18nProvider } from "@/i18n";
import { sessionsFixture } from "@/mocks/fixtures";

const mockToast = vi.hoisted(() => ({
  error: vi.fn(),
  success: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: mockToast,
}));

function renderWithProviders(node: ReactNode) {
  return render(<I18nProvider initialLocale="en-US">{node}</I18nProvider>);
}

function SessionsTableSelectionHarness() {
  const [selectedSessionIds, setSelectedSessionIds] = useState<string[]>([]);

  return (
    <SessionsTable
      sessions={sessionsFixture.sessions}
      listenCopyFormat="socks_url"
      isLoading={false}
      pendingCloseSessionIds={[]}
      closingSessionId={null}
      switchingSessionId={null}
      selectedSessionIds={selectedSessionIds}
      onSelectedSessionIdsChange={setSelectedSessionIds}
      onEditSession={vi.fn()}
      onUndoCloseSession={vi.fn()}
      onCloseSession={vi.fn()}
    />
  );
}

describe("SessionsTable", () => {
  const writeText = vi.fn().mockResolvedValue(undefined);

  beforeEach(() => {
    mockToast.error.mockReset();
    mockToast.success.mockReset();
    writeText.mockReset();
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
  });

  it("shows selected IP geography and removes the redundant port badge", () => {
    renderWithProviders(
      <SessionsTable
        sessions={sessionsFixture.sessions}
        listenCopyFormat="socks_url"
        isLoading={false}
        pendingCloseSessionIds={[]}
        closingSessionId={null}
        switchingSessionId={null}
        selectedSessionIds={[]}
        onSelectedSessionIdsChange={vi.fn()}
        onEditSession={vi.fn()}
        onUndoCloseSession={vi.fn()}
        onCloseSession={vi.fn()}
      />,
    );

    expect(screen.getByText("Japan / Tokyo / Chiyoda")).toBeInTheDocument();
    expect(screen.getByText("Japan / Osaka")).toBeInTheDocument();
    expect(screen.queryByText(/port 10080/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/port 10081/i)).not.toBeInTheDocument();
  });

  it("copies the listener address with the selected protocol format", async () => {
    const user = userEvent.setup();
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const baseSession = sessionsFixture.sessions[0];
    if (!baseSession) {
      throw new Error("Expected at least one session fixture.");
    }
    const sessions = {
      sessions: [
        {
          ...baseSession,
          listen: "0.0.0.0",
          bind_host: "0.0.0.0",
          display_host: "ops.example.test",
          display_address: "ops.example.test:10080",
        },
      ],
    };

    renderWithProviders(
      <SessionsTable
        sessions={sessions.sessions}
        listenCopyFormat="http_url"
        isLoading={false}
        pendingCloseSessionIds={[]}
        closingSessionId={null}
        switchingSessionId={null}
        selectedSessionIds={[]}
        onSelectedSessionIdsChange={vi.fn()}
        onEditSession={vi.fn()}
        onUndoCloseSession={vi.fn()}
        onCloseSession={vi.fn()}
      />,
    );

    await user.click(
      screen.getByRole("button", {
        name: /Copy proxy address for sess-A7c2Kp9LmQ4RsT1v/i,
      }),
    );

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith("http://ops.example.test:10080");
      expect(mockToast.success).toHaveBeenCalledWith("Copied proxy address");
    });
  });

  it("dims the row and exposes an undo action while close is pending", () => {
    const onUndoCloseSession = vi.fn();

    renderWithProviders(
      <SessionsTable
        sessions={sessionsFixture.sessions}
        listenCopyFormat="socks_url"
        isLoading={false}
        pendingCloseSessionIds={[sessionsFixture.sessions[0]?.session_id ?? ""]}
        closingSessionId={null}
        switchingSessionId={null}
        selectedSessionIds={[]}
        onSelectedSessionIdsChange={vi.fn()}
        onEditSession={vi.fn()}
        onUndoCloseSession={onUndoCloseSession}
        onCloseSession={vi.fn()}
      />,
    );

    const firstRow = screen.getByText("sess-A7c2Kp9LmQ4RsT1v").closest("tr");
    expect(firstRow).toHaveAttribute("data-close-state", "pending");

    const undoButton = screen.getByRole("button", { name: /^Undo$/i });
    expect(undoButton).toBeEnabled();
    expect(
      screen.getByRole("button", { name: /Edit proxy for sess-A7c2Kp9LmQ4RsT1v/i }),
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: /Copy proxy address for sess-A7c2Kp9LmQ4RsT1v/i }),
    ).toBeDisabled();

    undoButton.click();
    expect(onUndoCloseSession).toHaveBeenCalledWith("sess-A7c2Kp9LmQ4RsT1v");
  });

  it("selects multiple sessions by dragging through the checkbox column", () => {
    renderWithProviders(<SessionsTableSelectionHarness />);

    const firstCheckbox = screen.getByRole("checkbox", {
      name: /Select session sess-A7c2Kp9LmQ4RsT1v/i,
    });
    const secondCheckbox = screen.getByRole("checkbox", {
      name: /Select session sess-Q8n3Va1Zx5Mw2Lp7/i,
    });
    const firstCell = firstCheckbox.closest("td");
    const secondCell = secondCheckbox.closest("td");
    if (!firstCell || !secondCell) {
      throw new Error("Expected session selection cells.");
    }

    fireEvent.pointerDown(firstCell, { pointerType: "mouse", button: 0 });
    fireEvent.pointerEnter(secondCell, { pointerType: "mouse", button: 0 });
    fireEvent.pointerUp(window);

    expect(firstCheckbox).toBeChecked();
    expect(secondCheckbox).toBeChecked();
  });

  it("selects multiple sessions by touch dragging through the checkbox column", () => {
    renderWithProviders(<SessionsTableSelectionHarness />);

    const firstCheckbox = screen.getByRole("checkbox", {
      name: /Select session sess-A7c2Kp9LmQ4RsT1v/i,
    });
    const secondCheckbox = screen.getByRole("checkbox", {
      name: /Select session sess-Q8n3Va1Zx5Mw2Lp7/i,
    });
    const firstCell = firstCheckbox.closest("td");
    const secondCell = secondCheckbox.closest("td");
    if (!firstCell || !secondCell) {
      throw new Error("Expected session selection cells.");
    }

    const originalElementFromPoint = document.elementFromPoint;
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => secondCell),
    });

    fireEvent.pointerDown(firstCell, { pointerType: "touch", clientX: 10, clientY: 10 });
    fireEvent.pointerMove(firstCell, { pointerType: "touch", clientX: 10, clientY: 48 });
    fireEvent.pointerUp(window);

    expect(firstCheckbox).toBeChecked();
    expect(secondCheckbox).toBeChecked();

    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: originalElementFromPoint,
    });
  });

  it("toggles one session when clicking its checkbox", async () => {
    const user = userEvent.setup();
    renderWithProviders(<SessionsTableSelectionHarness />);

    const firstCheckbox = screen.getByRole("checkbox", {
      name: /Select session sess-A7c2Kp9LmQ4RsT1v/i,
    });

    await user.click(firstCheckbox);

    expect(firstCheckbox).toBeChecked();
  });

  it("extends session selection with shift range selection", () => {
    renderWithProviders(<SessionsTableSelectionHarness />);

    const firstCheckbox = screen.getByRole("checkbox", {
      name: /Select session sess-A7c2Kp9LmQ4RsT1v/i,
    });
    const secondCheckbox = screen.getByRole("checkbox", {
      name: /Select session sess-Q8n3Va1Zx5Mw2Lp7/i,
    });
    const firstCell = firstCheckbox.closest("td");
    const secondCell = secondCheckbox.closest("td");
    if (!firstCell || !secondCell) {
      throw new Error("Expected session selection cells.");
    }

    fireEvent.pointerDown(firstCell, { pointerType: "mouse", button: 0 });
    fireEvent.pointerUp(window);
    fireEvent.pointerDown(secondCell, { pointerType: "mouse", button: 0, shiftKey: true });
    fireEvent.pointerUp(window);

    expect(firstCheckbox).toBeChecked();
    expect(secondCheckbox).toBeChecked();
  });
});
