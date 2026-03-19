import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { AccessControlCard } from "@/features/overview/components/AccessControlCard";

describe("AccessControlCard", () => {
  it("creates and revokes profile keys through callbacks", async () => {
    const user = userEvent.setup();
    const onCreateApiKey = vi.fn().mockResolvedValue(undefined);
    const onRevokeApiKey = vi.fn().mockResolvedValue(undefined);

    render(
      <AccessControlCard
        identity={{
          authenticated: true,
          principal_type: "human",
          subject: "admin@example.com",
          email: "admin@example.com",
          groups: ["admins"],
          is_admin: true,
        }}
        apiKeys={[
          {
            key_id: "key-1",
            profile_id: "edge-jp",
            name: "deploy-bot",
            prefix: "pbk_key-1_prefix",
            created_by: "admin@example.com",
            created_at: 1_742_447_800,
            last_used_at: null,
            revoked_at: null,
          },
        ]}
        onCreateApiKey={onCreateApiKey}
        onRevokeApiKey={onRevokeApiKey}
      />,
    );

    await user.type(screen.getByLabelText("API key name"), "ci-runner");
    await user.click(screen.getByRole("button", { name: /create key/i }));
    expect(onCreateApiKey).toHaveBeenCalledWith("ci-runner");

    await user.click(screen.getByRole("button", { name: /revoke/i }));
    expect(onRevokeApiKey).toHaveBeenCalledWith("key-1");
  });
});
