import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SearchableMultiSelect } from "@/components/SearchableMultiSelect";
import { I18nProvider } from "@/i18n";
import type { SessionOptionItem } from "@/lib/types";

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

describe("SearchableMultiSelect", () => {
  beforeEach(() => {
    installLocalStorage();
  });

  it("localizes loading and error states under zh-CN", async () => {
    const user = userEvent.setup();
    let rejectSearch: ((reason?: unknown) => void) | undefined;
    const onSearch = vi.fn(
      () =>
        new Promise<SessionOptionItem[]>((_, reject) => {
          rejectSearch = reject;
        }),
    );

    render(
      <I18nProvider initialLocale="zh-CN">
        <SearchableMultiSelect
          id="searchable-ip"
          label="IP"
          placeholder="搜索并选择 IP"
          searchPlaceholder="搜索 IP"
          emptyText="没有匹配的 IP"
          values={[]}
          onChange={vi.fn()}
          onSearch={onSearch}
        />
      </I18nProvider>,
    );

    await user.click(screen.getByRole("combobox", { name: "IP" }));
    expect(await screen.findByText("正在加载选项…")).toBeInTheDocument();

    rejectSearch?.(new Error("boom"));

    await waitFor(() => {
      expect(screen.getByText("无法加载选项")).toBeInTheDocument();
    });
  });
});
