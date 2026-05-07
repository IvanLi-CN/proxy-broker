import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { AccessControlCard } from "@/features/overview/components/AccessControlCard";

describe("AccessControlCard", () => {
  it("creates selected-project keys from the current project and revokes existing keys", async () => {
    const user = userEvent.setup();
    const onCreateApiKey = vi.fn().mockResolvedValue(undefined);
    const onRevokeApiKey = vi.fn().mockResolvedValue(undefined);

    render(
      <AccessControlCard
        currentProjectId="edge-jp"
        availableProjects={["default", "edge-jp", "lab-us"]}
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
            key_id: "key-Q4w8Er2Ty6Ui1Op5",
            project_id: "edge-jp",
            name: "deploy-bot",
            prefix: "pbk_key-Q4w8Er2Ty6",
            created_by: "admin@example.com",
            owner_subject: "admin@example.com",
            project_scope: {
              kind: "selected_projects",
              project_ids: ["edge-jp"],
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
        project_scope: {
          kind: "selected_projects",
          project_ids: ["edge-jp"],
        },
      });
    });

    await user.click(screen.getByRole("button", { name: /revoke/i }));
    expect(onRevokeApiKey).toHaveBeenCalledWith("key-Q4w8Er2Ty6Ui1Op5");
  });

  it("creates all-project keys when the checkbox is enabled", async () => {
    const user = userEvent.setup();
    const onCreateApiKey = vi.fn().mockResolvedValue(undefined);

    render(
      <AccessControlCard
        currentProjectId="edge-jp"
        availableProjects={["default", "edge-jp", "lab-us"]}
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
    await user.click(screen.getByRole("checkbox", { name: /allow all projects/i }));
    await user.click(screen.getByRole("button", { name: /create key/i }));

    await waitFor(() => {
      expect(onCreateApiKey).toHaveBeenCalledWith({
        name: "fleet-bot",
        project_scope: {
          kind: "all_projects",
        },
      });
    });
  });

  it("lets admins extend selected-project scope beyond the current project", async () => {
    const user = userEvent.setup();
    const onCreateApiKey = vi.fn().mockResolvedValue(undefined);

    render(
      <AccessControlCard
        currentProjectId="edge-jp"
        availableProjects={["default", "edge-jp", "lab-us"]}
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
            key_id: "key-L7k3Nm9Qa2Ws5Ed8",
            project_id: null,
            name: "multi-bot",
            prefix: "pbk_key-L7k3Nm9Qa2",
            created_by: "admin@example.com",
            owner_subject: "admin@example.com",
            project_scope: {
              kind: "selected_projects",
              project_ids: ["edge-jp", "lab-us"],
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

    await user.click(screen.getByRole("combobox", { name: /available projects/i }));
    await user.click(await screen.findByText("lab-us"));
    await user.click(screen.getByRole("combobox", { name: /available projects/i }));
    await user.type(screen.getByLabelText("API key name"), "multi-bot");
    await waitFor(() => {
      expect(screen.getByLabelText("API key name")).toHaveValue("multi-bot");
    });
    await user.click(screen.getByRole("button", { name: /create key/i }));

    await waitFor(() => {
      expect(onCreateApiKey).toHaveBeenCalledWith({
        name: "multi-bot",
        project_scope: {
          kind: "selected_projects",
          project_ids: ["edge-jp", "lab-us"],
        },
      });
    });
  });
});
