import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { I18nProvider, resolveInitialLocale, useI18n } from "@/i18n";

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
    clear: () => {
      store.clear();
    },
  };

  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: storage,
  });
}

function setNavigatorLanguages(...languages: string[]) {
  Object.defineProperty(window.navigator, "language", {
    configurable: true,
    value: languages[0] ?? "en-US",
  });
  Object.defineProperty(window.navigator, "languages", {
    configurable: true,
    value: languages,
  });
}

function LocaleProbe() {
  const { locale, setLocale, t } = useI18n();

  return (
    <div>
      <div>{locale}</div>
      <div>{t("Language")}</div>
      <button type="button" onClick={() => setLocale("zh-CN")}>
        set-zh
      </button>
    </div>
  );
}

describe("i18n locale resolution", () => {
  beforeEach(() => {
    installLocalStorage();
    window.localStorage.removeItem(localeStorageKey);
    document.documentElement.lang = "";
    setNavigatorLanguages("en-US");
  });

  afterEach(() => {
    window.localStorage.removeItem(localeStorageKey);
    document.documentElement.lang = "";
  });

  it("prefers a persisted locale over browser language", () => {
    window.localStorage.setItem(localeStorageKey, "en-US");
    setNavigatorLanguages("zh-CN", "en-US");

    expect(resolveInitialLocale()).toBe("en-US");
  });

  it("falls back to zh-CN for Chinese browser locales", () => {
    setNavigatorLanguages("zh-Hans-CN", "en-US");

    expect(resolveInitialLocale()).toBe("zh-CN");
  });

  it("updates document language and storage when the locale changes", async () => {
    const user = userEvent.setup();

    render(
      <I18nProvider initialLocale="en-US">
        <LocaleProbe />
      </I18nProvider>,
    );

    expect(screen.getByText("Language")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "set-zh" }));

    expect(document.documentElement.lang).toBe("zh-Hans");
    expect(window.localStorage.getItem(localeStorageKey)).toBe("zh-CN");
    expect(screen.getByText("语言")).toBeInTheDocument();
  });
});
