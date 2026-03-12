import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";

describe("ActionResponsePanel", () => {
  it("renders title, description, and bullets", () => {
    render(
      <ActionResponsePanel
        title="Subscription warnings"
        description="The backend returned advisory warnings."
        tone="warning"
        bullets={["JP-Relay-02 reused cached IP", "SG-Relay-01 skipped probe"]}
      />,
    );

    expect(screen.getByText("Subscription warnings")).toBeInTheDocument();
    expect(screen.getByText("The backend returned advisory warnings.")).toBeInTheDocument();
    expect(screen.getByText("JP-Relay-02 reused cached IP")).toBeInTheDocument();
    expect(screen.getByText("SG-Relay-01 skipped probe")).toBeInTheDocument();
  });
});
