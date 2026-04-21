import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TaskFiltersBar } from "@/features/tasks/components/TaskFiltersBar";
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

function renderBar() {
  return render(
    <I18nProvider initialLocale="en-US">
      <TaskFiltersBar
        scope="current"
        runningOnly={false}
        onKindChange={vi.fn()}
        onRunningOnlyChange={vi.fn()}
        onScopeChange={vi.fn()}
        onStatusChange={vi.fn()}
        onTriggerChange={vi.fn()}
      />
    </I18nProvider>,
  );
}

describe("TaskFiltersBar", () => {
  beforeEach(() => {
    installLocalStorage();
  });

  it("includes the new proxy task kinds", async () => {
    const user = userEvent.setup();
    renderBar();

    const comboBoxes = screen.getAllByRole("combobox");
    const kindComboBox = comboBoxes.at(1);
    expect(kindComboBox).toBeDefined();
    if (!kindComboBox) {
      throw new Error("task kind combobox missing");
    }
    await user.click(kindComboBox);

    expect(await screen.findByText("Proxy metadata refresh")).toBeInTheDocument();
    expect(screen.getByText("Proxy latency probe")).toBeInTheDocument();
  });

  it("includes the operator trigger", async () => {
    const user = userEvent.setup();
    renderBar();

    const comboBoxes = screen.getAllByRole("combobox");
    const triggerComboBox = comboBoxes.at(3);
    expect(triggerComboBox).toBeDefined();
    if (!triggerComboBox) {
      throw new Error("task trigger combobox missing");
    }
    await user.click(triggerComboBox);

    expect(await screen.findByText("Operator")).toBeInTheDocument();
  });
});
