import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SessionSelectionModeSwitch } from "@/features/sessions/components/SessionSelectionModeSwitch";
import { I18nProvider } from "@/i18n";

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

describe("SessionSelectionModeSwitch", () => {
  beforeEach(() => {
    installLocalStorage();
  });

  it("renders English labels under en-US", () => {
    render(
      <I18nProvider initialLocale="en-US">
        <SessionSelectionModeSwitch value="geo" onChange={vi.fn()} />
      </I18nProvider>,
    );

    expect(screen.getByRole("tablist", { name: "Targeting mode" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Any" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Country / region" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "IP" })).toBeInTheDocument();
  });

  it("renders Simplified Chinese labels under zh-CN", () => {
    render(
      <I18nProvider initialLocale="zh-CN">
        <SessionSelectionModeSwitch value="geo" onChange={vi.fn()} />
      </I18nProvider>,
    );

    expect(screen.getByRole("tablist", { name: "定位方式" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "不限" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "国家/地区" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "IP" })).toBeInTheDocument();
  });
});
