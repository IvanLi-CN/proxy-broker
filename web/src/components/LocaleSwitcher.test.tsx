import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { LocaleSwitcher } from "@/components/LocaleSwitcher";
import { I18nProvider } from "@/i18n";

const localeStorageKey = "proxy-broker.locale";

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

describe("LocaleSwitcher", () => {
  beforeEach(() => {
    installLocalStorage();
    window.localStorage.removeItem(localeStorageKey);
    document.documentElement.lang = "";
  });

  it("switches to zh-CN and persists the selection", async () => {
    const user = userEvent.setup();

    render(
      <I18nProvider initialLocale="en-US">
        <LocaleSwitcher />
      </I18nProvider>,
    );

    await user.click(screen.getByRole("combobox", { name: "Operator console language" }));
    await user.click(screen.getByText("Simplified Chinese"));

    await waitFor(() => {
      expect(document.documentElement.lang).toBe("zh-CN");
    });

    expect(window.localStorage.getItem(localeStorageKey)).toBe("zh-CN");
    expect(screen.getByText("语言")).toBeInTheDocument();
    expect(screen.getByText("简体中文")).toBeInTheDocument();
  });
});
