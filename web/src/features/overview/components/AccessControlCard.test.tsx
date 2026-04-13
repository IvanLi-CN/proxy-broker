import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { AccessControlCard } from "@/features/overview/components/AccessControlCard";

describe("AccessControlCard", () => {
  it("creates selected-profile keys from the current profile and revokes existing keys", async () => {
    const user = userEvent.setup();
    const onCreateApiKey = vi.fn().mockResolvedValue(undefined);
    const onRevokeApiKey = vi.fn().mockResolvedValue(undefined);

    render(
      <AccessControlCard
        currentProfileId="edge-jp"
        availableProfiles={["default", "edge-jp", "lab-us"]}
        currentUser={{
          status: "resolved",
          identity: {
            authenticated: true,
            principal_type: "human",
            subject: "admin@example.com",
            email: "admin@example.com",
            groups: ["admins"],
            is_admin: true,
          },
        }}
        apiKeys={[
          {
            key_id: "key-1",
            profile_id: "edge-jp",
            name: "deploy-bot",
            prefix: "pbk_key-1_prefix",
            created_by: "admin@example.com",
            owner_subject: "admin@example.com",
            profile_scope: {
              kind: "selected_profiles",
              profile_ids: ["edge-jp"],
            },
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
    await waitFor(() => {
      expect(screen.getByLabelText("API key name")).toHaveValue("ci-runner");
    });
    await user.click(screen.getByRole("button", { name: /create key/i }));
    await waitFor(() => {
      expect(onCreateApiKey).toHaveBeenCalledWith({
        name: "ci-runner",
        profile_scope: {
          kind: "selected_profiles",
          profile_ids: ["edge-jp"],
        },
      });
    });

    await user.click(screen.getByRole("button", { name: /revoke/i }));
    expect(onRevokeApiKey).toHaveBeenCalledWith("key-1");
  });

  it("creates all-profile keys when the checkbox is enabled", async () => {
    const user = userEvent.setup();
    const onCreateApiKey = vi.fn().mockResolvedValue(undefined);

    render(
      <AccessControlCard
        currentProfileId="edge-jp"
        availableProfiles={["default", "edge-jp", "lab-us"]}
        currentUser={{
          status: "resolved",
          identity: {
            authenticated: true,
            principal_type: "human",
            subject: "admin@example.com",
            email: "admin@example.com",
            groups: ["admins"],
            is_admin: true,
          },
        }}
        apiKeys={[]}
        onCreateApiKey={onCreateApiKey}
        onRevokeApiKey={vi.fn()}
      />,
    );

    await user.type(screen.getByLabelText("API key name"), "fleet-bot");
    await waitFor(() => {
      expect(screen.getByLabelText("API key name")).toHaveValue("fleet-bot");
    });
    await user.click(screen.getByRole("checkbox", { name: /allow all profiles/i }));
    await user.click(screen.getByRole("button", { name: /create key/i }));

    await waitFor(() => {
      expect(onCreateApiKey).toHaveBeenCalledWith({
        name: "fleet-bot",
        profile_scope: {
          kind: "all_profiles",
        },
      });
    });
  });

  it("lets admins extend selected-profile scope beyond the current profile", async () => {
    const user = userEvent.setup();
    const onCreateApiKey = vi.fn().mockResolvedValue(undefined);

    render(
      <AccessControlCard
        currentProfileId="edge-jp"
        availableProfiles={["default", "edge-jp", "lab-us"]}
        currentUser={{
          status: "resolved",
          identity: {
            authenticated: true,
            principal_type: "human",
            subject: "admin@example.com",
            email: "admin@example.com",
            groups: ["admins"],
            is_admin: true,
          },
        }}
        apiKeys={[
          {
            key_id: "key-1",
            profile_id: null,
            name: "multi-bot",
            prefix: "pbk_key-1_prefix",
            created_by: "admin@example.com",
            owner_subject: "admin@example.com",
            profile_scope: {
              kind: "selected_profiles",
              profile_ids: ["edge-jp", "lab-us"],
            },
            created_at: 1_742_447_800,
            last_used_at: null,
            revoked_at: null,
          },
        ]}
        onCreateApiKey={onCreateApiKey}
        onRevokeApiKey={vi.fn()}
      />,
    );

    expect(screen.getByText("edge-jp / lab-us")).toBeInTheDocument();

    await user.click(screen.getByRole("combobox", { name: /available profiles/i }));
    await user.click(await screen.findByText("lab-us"));
    await user.click(screen.getByRole("combobox", { name: /available profiles/i }));
    await user.type(screen.getByLabelText("API key name"), "multi-bot");
    await waitFor(() => {
      expect(screen.getByLabelText("API key name")).toHaveValue("multi-bot");
    });
    await user.click(screen.getByRole("button", { name: /create key/i }));

    await waitFor(() => {
      expect(onCreateApiKey).toHaveBeenCalledWith({
        name: "multi-bot",
        profile_scope: {
          kind: "selected_profiles",
          profile_ids: ["edge-jp", "lab-us"],
        },
      });
    });
  });
});
