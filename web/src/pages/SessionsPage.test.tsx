import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { sessionCopyAddressFormatStorageKey } from "@/features/sessions/hooks/use-session-copy-address-format";
import { I18nProvider } from "@/i18n";
import { sessionNodeOptionsFixture, sessionsFixture } from "@/mocks/fixtures";
import { SessionsPage } from "@/pages/SessionsPage";

const mockToast = vi.hoisted(() => ({
  error: vi.fn(),
  success: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: mockToast,
}));

function installLocalStorage() {
  const store = new Map<string, string>();
  const storage = {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
    removeItem: (key: string) => {
      store.delete(key);
    },
  };

  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: storage,
  });
}

function renderWithProviders(node: ReactNode) {
  return render(<I18nProvider initialLocale="en-US">{node}</I18nProvider>);
}

describe("SessionsPage", () => {
  beforeEach(() => {
    installLocalStorage();
    mockToast.error.mockReset();
    mockToast.success.mockReset();
    window.localStorage.removeItem(sessionCopyAddressFormatStorageKey);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("hydrates the persisted copy format and writes the next selection back to local storage", async () => {
    window.localStorage.setItem(sessionCopyAddressFormatStorageKey, "http_url");
    const user = userEvent.setup();

    renderWithProviders(
      <SessionsPage
        sessions={sessionsFixture.sessions}
        sessionsLoading={false}
        openError={null}
        batchError={null}
        switchError={null}
        openResponse={null}
        batchResponse={null}
        switchedSessionId={null}
        opening={false}
        batchOpening={false}
        suggestedPort={10080}
        closingSessionId={null}
        switchingSessionId={null}
        onOpenSession={vi.fn()}
        onOpenBatch={vi.fn()}
        onUpdateSessionNode={vi.fn()}
        searchSessionOptions={vi.fn(async () => [])}
        searchSessionNodeOptions={vi.fn(async () => sessionNodeOptionsFixture.items)}
        onCloseSession={vi.fn()}
        onResetCreateState={vi.fn()}
        onResetSwitchState={vi.fn()}
      />,
    );

    expect(screen.getByRole("radio", { name: /HTTP URI/i })).toHaveAttribute("data-state", "on");
    expect(screen.queryByText("Will copy")).not.toBeInTheDocument();
    expect(
      screen.queryByText("Preview appears after the first session opens."),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: /Host:port/i }));

    expect(window.localStorage.getItem(sessionCopyAddressFormatStorageKey)).toBe("host_port");
    expect(screen.getByRole("radio", { name: /Host:port/i })).toHaveAttribute("data-state", "on");
  });

  it("greys the row, swaps close to undo, and delays removal for 10 seconds", async () => {
    vi.useFakeTimers();
    const onCloseSession = vi.fn().mockResolvedValue(undefined);

    renderWithProviders(
      <SessionsPage
        sessions={sessionsFixture.sessions}
        sessionsLoading={false}
        openError={null}
        batchError={null}
        switchError={null}
        openResponse={null}
        batchResponse={null}
        switchedSessionId={null}
        opening={false}
        batchOpening={false}
        suggestedPort={10080}
        closingSessionId={null}
        switchingSessionId={null}
        onOpenSession={vi.fn()}
        onOpenBatch={vi.fn()}
        onUpdateSessionNode={vi.fn()}
        searchSessionOptions={vi.fn(async () => [])}
        searchSessionNodeOptions={vi.fn(async () => sessionNodeOptionsFixture.items)}
        onCloseSession={onCloseSession}
        onResetCreateState={vi.fn()}
        onResetSwitchState={vi.fn()}
      />,
    );

    const firstCloseButton = screen.getAllByRole("button", { name: /^Close$/i })[0];
    if (!firstCloseButton) {
      throw new Error("Expected at least one close button.");
    }

    await act(async () => {
      fireEvent.click(firstCloseButton);
    });

    const firstRow = screen.getByText("sess-A7c2Kp9LmQ4RsT1v").closest("tr");
    expect(firstRow).toHaveAttribute("data-close-state", "pending");
    expect(screen.getByRole("button", { name: /^Undo$/i })).toBeVisible();
    expect(onCloseSession).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(9_000);
    });
    expect(onCloseSession).not.toHaveBeenCalled();
    expect(screen.getByText("sess-A7c2Kp9LmQ4RsT1v")).toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(onCloseSession).toHaveBeenCalledWith("sess-A7c2Kp9LmQ4RsT1v");
    expect(screen.queryByText("sess-A7c2Kp9LmQ4RsT1v")).not.toBeInTheDocument();
  });

  it("cancels the delayed close when undo is clicked", async () => {
    vi.useFakeTimers();
    const onCloseSession = vi.fn().mockResolvedValue(undefined);

    renderWithProviders(
      <SessionsPage
        sessions={sessionsFixture.sessions}
        sessionsLoading={false}
        openError={null}
        batchError={null}
        switchError={null}
        openResponse={null}
        batchResponse={null}
        switchedSessionId={null}
        opening={false}
        batchOpening={false}
        suggestedPort={10080}
        closingSessionId={null}
        switchingSessionId={null}
        onOpenSession={vi.fn()}
        onOpenBatch={vi.fn()}
        onUpdateSessionNode={vi.fn()}
        searchSessionOptions={vi.fn(async () => [])}
        searchSessionNodeOptions={vi.fn(async () => sessionNodeOptionsFixture.items)}
        onCloseSession={onCloseSession}
        onResetCreateState={vi.fn()}
        onResetSwitchState={vi.fn()}
      />,
    );

    const firstCloseButton = screen.getAllByRole("button", { name: /^Close$/i })[0];
    if (!firstCloseButton) {
      throw new Error("Expected at least one close button.");
    }

    await act(async () => {
      fireEvent.click(firstCloseButton);
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /^Undo$/i }));
    });

    const firstRow = screen.getByText("sess-A7c2Kp9LmQ4RsT1v").closest("tr");
    expect(firstRow).toHaveAttribute("data-close-state", "idle");

    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });
    expect(onCloseSession).not.toHaveBeenCalled();
    expect(screen.getByText("sess-A7c2Kp9LmQ4RsT1v")).toBeInTheDocument();
  });
});
