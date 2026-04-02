import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { NodesTable } from "@/features/nodes/components/NodesTable";
import { I18nProvider } from "@/i18n";
import { nodesFixture } from "@/mocks/fixtures";

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

describe("NodesTable", () => {
  beforeEach(() => {
    installLocalStorage();
  });

  it("renders grouping headers for grouped views", () => {
    render(
      <I18nProvider initialLocale="en-US">
        <NodesTable
          items={nodesFixture.items}
          viewMode="group_by_region"
          selectedIds={[]}
          onToggleSelect={vi.fn()}
          onToggleSelectAll={vi.fn()}
        />
      </I18nProvider>,
    );

    expect(screen.getByText("Japan / Tokyo / Chiyoda")).toBeInTheDocument();
    expect(screen.getByText("United States / California / San Jose")).toBeInTheDocument();
  });

  it("toggles a row selection", async () => {
    const user = userEvent.setup();
    const onToggleSelect = vi.fn();

    render(
      <I18nProvider initialLocale="en-US">
        <NodesTable
          items={nodesFixture.items}
          viewMode="flat"
          selectedIds={[]}
          onToggleSelect={onToggleSelect}
          onToggleSelectAll={vi.fn()}
        />
      </I18nProvider>,
    );

    const checkboxes = screen.getAllByRole("checkbox");
    const rowCheckbox = checkboxes[1];
    expect(rowCheckbox).toBeDefined();
    if (!rowCheckbox) {
      throw new Error("expected a row checkbox");
    }
    await user.click(rowCheckbox);

    expect(onToggleSelect).toHaveBeenCalledWith("JP-Tokyo-Entry", true);
  });
});
