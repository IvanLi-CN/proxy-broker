import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

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

    expect(screen.getByRole("combobox", { name: /Copy address format/i })).toHaveTextContent(
      /HTTP address/i,
    );

    await user.click(screen.getByRole("combobox", { name: /Copy address format/i }));
    await user.click(screen.getByRole("option", { name: /SOCKS address/i }));

    expect(window.localStorage.getItem(sessionCopyAddressFormatStorageKey)).toBe("socks_url");
  });
});
