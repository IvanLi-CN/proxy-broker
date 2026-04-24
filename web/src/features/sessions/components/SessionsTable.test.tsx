import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
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

  it("copies the listener address with the selected protocol format", async () => {
    const user = userEvent.setup();
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    renderWithProviders(
      <SessionsTable
        sessions={sessionsFixture.sessions}
        listenCopyFormat="http_url"
        isLoading={false}
        closingSessionId={null}
        switchingSessionId={null}
        onEditSession={vi.fn()}
        onCloseSession={vi.fn()}
      />,
    );

    await user.click(
      screen.getByRole("button", {
        name: /Copy proxy address for sess-A7c2Kp9LmQ4RsT1v/i,
      }),
    );

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith("http://127.0.0.1:10080");
      expect(mockToast.success).toHaveBeenCalledWith("Copied proxy address");
    });
  });
});
