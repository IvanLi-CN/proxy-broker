import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { NodesFiltersBar } from "@/features/nodes/components/NodesFiltersBar";
import { I18nProvider } from "@/i18n";
import { defaultNodeFilterState } from "@/lib/nodes-view";

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

describe("NodesFiltersBar", () => {
  it("localizes select values and options in zh-CN", async () => {
    const user = userEvent.setup();
    installLocalStorage();

    render(
      <I18nProvider initialLocale="zh-CN">
        <NodesFiltersBar state={defaultNodeFilterState} onChange={vi.fn()} onReset={vi.fn()} />
      </I18nProvider>,
    );

    expect(screen.getByText("全部探测状态")).toBeInTheDocument();
    expect(screen.getByText("升序")).toBeInTheDocument();

    const [probeStateSelect] = screen.getAllByRole("combobox");
    expect(probeStateSelect).toBeDefined();
    if (!probeStateSelect) {
      throw new Error("expected a probe-state combobox");
    }

    await user.click(probeStateSelect);
    expect(screen.getByRole("option", { name: "可达" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "不可达" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "未探测" })).toBeInTheDocument();
  });
});
