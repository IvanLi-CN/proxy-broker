import { TooltipProvider } from "@radix-ui/react-tooltip";
import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { OpenBatchForm } from "@/features/sessions/components/OpenBatchForm";
import { OpenSessionForm } from "@/features/sessions/components/OpenSessionForm";
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

function renderWithProviders(node: ReactNode) {
  return render(
    <I18nProvider initialLocale="en-US">
      <TooltipProvider>{node}</TooltipProvider>
    </I18nProvider>,
  );
}

describe("Sessions forms localization", () => {
  beforeEach(() => {
    installLocalStorage();
  });

  it("renders the single-open controls in English under en-US", () => {
    renderWithProviders(
      <OpenSessionForm
        isPending={false}
        suggestedPort={10080}
        defaultAdvancedOpen
        initialValues={{ selectionMode: "geo", countryCodes: ["JP"], cities: ["Osaka"] }}
        onSubmit={vi.fn()}
      />,
    );

    expect(screen.getByText("Targeting mode")).toBeInTheDocument();
    expect(screen.getByText("Country")).toBeInTheDocument();
    expect(screen.getByText("Region / city")).toBeInTheDocument();
    expect(screen.getByText("Port")).toBeInTheDocument();
    expect(screen.getByText("Selection order")).toBeInTheDocument();
    expect(screen.getByText("Exclude IP")).toBeInTheDocument();
  });

  it("renders the batch row controls in English under en-US", () => {
    renderWithProviders(
      <OpenBatchForm
        isPending={false}
        suggestedPort={10080}
        defaultAdvancedOpen
        initialRequests={[
          {
            selectionMode: "geo",
            desiredPort: "",
            countryCodes: ["JP"],
            cities: ["Osaka"],
            specifiedIps: [],
            excludedIps: [],
            sortMode: "lru",
          },
        ]}
        onSubmit={vi.fn()}
      />,
    );

    expect(screen.getAllByText("Targeting mode")[0]).toBeInTheDocument();
    expect(screen.getByText("Country")).toBeInTheDocument();
    expect(screen.getByText("Region / city")).toBeInTheDocument();
    expect(screen.getByText("Port")).toBeInTheDocument();
    expect(screen.getByText("Selection order")).toBeInTheDocument();
    expect(screen.getByText("Exclude IP")).toBeInTheDocument();
  });
});
