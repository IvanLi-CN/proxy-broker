import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { CurrentUserSummary } from "@/components/CurrentUserSummary";

describe("CurrentUserSummary", () => {
  it("renders anonymous state explicitly", () => {
    render(<CurrentUserSummary currentUser={{ status: "anonymous" }} />);

    expect(screen.getByText("Anonymous browser session")).toBeInTheDocument();
    expect(screen.getByText("anonymous")).toBeInTheDocument();
  });

  it("renders api key metadata for machine identities", () => {
    render(
      <CurrentUserSummary
        currentUser={{
          status: "resolved",
          identity: {
            authenticated: true,
            principal_type: "api_key",
            subject: "deploy-bot",
            groups: [],
            is_admin: false,
            api_key_id: "key-Q4w8Er2Ty6Ui1Op5",
            api_key_owner_subject: "admin@example.com",
            api_key_project_scope: {
              kind: "selected_projects",
              project_ids: ["default", "edge-jp"],
            },
          },
        }}
      />,
    );

    expect(
      screen.getByText("Machine principal resolved from an owner-scoped API key."),
    ).toBeInTheDocument();
    expect(screen.getByText("API key ID: key-Q4w8Er2Ty6Ui1Op5")).toBeInTheDocument();
    expect(screen.getByText("Owner admin@example.com")).toBeInTheDocument();
    expect(screen.getByText("Scope default / edge-jp")).toBeInTheDocument();
  });
});
