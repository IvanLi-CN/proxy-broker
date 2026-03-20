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
            profile_id: "default",
            api_key_id: "key-7",
          },
        }}
      />,
    );

    expect(
      screen.getByText("Machine principal resolved from a profile-scoped API key."),
    ).toBeInTheDocument();
    expect(screen.getByText("API key ID: key-7")).toBeInTheDocument();
    expect(screen.getByText("Bound profile: default")).toBeInTheDocument();
  });
});
