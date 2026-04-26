import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { AppShell } from "@/components/AppShell";
import { sessionCopyAddressFormatStorageKey } from "@/features/sessions/hooks/use-session-copy-address-format";
import type { SessionListItem } from "@/lib/types";
import { sessionNodeOptionsFixture, sessionsFixture } from "@/mocks/fixtures";
import { SessionsPage } from "@/pages/SessionsPage";

const longSessionList: SessionListItem[] = Array.from({ length: 28 }, (_, index) => {
  const source = sessionsFixture.sessions[index % sessionsFixture.sessions.length];
  if (!source) {
    throw new Error("Expected session fixtures.");
  }

  const port = 10080 + index;
  const regionNames = ["Tokyo", "Osaka", "Singapore", "Seoul", "California", "Sydney"];
  const cityNames = ["Chiyoda", "Osaka", "Singapore", "Seoul", "San Jose", "Sydney"];
  const proxyNames = [
    "JP-Tokyo-Entry",
    "JP-Osaka-Edge",
    "SG-Media-Relay",
    "KR-Seoul-BGP",
    "US-LosAngeles-Fast",
    "AU-Sydney-HY2",
  ];

  return {
    ...source,
    session_id: `sess-long-${String(index + 1).padStart(2, "0")}-${source.session_id.slice(-6)}`,
    listen: `127.0.0.1:${port}`,
    display_address: `127.0.0.1:${port}`,
    port,
    proxy_name: proxyNames[index % proxyNames.length] ?? source.proxy_name,
    selected_ip: `203.0.113.${10 + index}`,
    created_at: source.created_at + index * 60,
    region_name: regionNames[index % regionNames.length],
    city: cityNames[index % cityNames.length],
  };
});

const meta = {
  title: "Pages/SessionsPage",
  component: SessionsPage,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    initialEntries: ["/sessions"],
    docs: {
      description: {
        component:
          "Session route inside the real app shell, keeping the session list as the primary surface and moving create/switch flows into dialogs.",
      },
    },
  },
  render: (args) => {
    if (typeof window !== "undefined") {
      window.localStorage.removeItem(sessionCopyAddressFormatStorageKey);
    }

    return (
      <AppShell
        profileId="default"
        profiles={["default", "edge-jp", "lab-us"]}
        profilesLoading={false}
        profilesCreating={false}
        profilesError={null}
        healthStatus="ok"
        currentUser={{
          status: "resolved",
          identity: {
            authenticated: true,
            principal_type: "human",
            subject: "admin@example.com",
            email: "admin@example.com",
            groups: ["admins", "ops"],
            is_admin: true,
          },
        }}
        onProfileIdChange={() => undefined}
        onCreateProfile={async (value: string) => value}
        onRetryProfiles={() => undefined}
      >
        <SessionsPage {...args} />
      </AppShell>
    );
  },
  args: {
    sessions: sessionsFixture.sessions,
    sessionsLoading: false,
    openError: null,
    batchError: null,
    switchError: null,
    openResponse: null,
    batchResponse: null,
    switchedSessionId: null,
    opening: false,
    batchOpening: false,
    suggestedPort: 10080,
    closingSessionId: null,
    switchingSessionId: null,
    onOpenSession: fn(),
    onOpenBatch: fn(),
    onUpdateSessionNode: fn(),
    searchSessionOptions: fn(async () => []),
    searchSessionNodeOptions: fn(async () => sessionNodeOptionsFixture.items),
    onCloseSession: fn(),
    onResetCreateState: fn(),
    onResetSwitchState: fn(),
  },
} satisfies Meta<typeof SessionsPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};

export const ZhCN: Story = {
  args: {
    sessions: longSessionList,
  },
  globals: {
    locale: "zh-CN",
  },
};

export const LongList: Story = {
  args: {
    sessions: longSessionList,
  },
  globals: {
    locale: "zh-CN",
  },
};

export const EmptyState: Story = {
  args: {
    sessions: [],
  },
};

export const ClosingState: Story = {
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const closeButtons = await canvas.findAllByRole("button", { name: /^Close$/i });
    const firstCloseButton = closeButtons[0];
    if (!firstCloseButton) {
      throw new Error("Expected at least one close button in the session table.");
    }

    await userEvent.click(firstCloseButton);
    const pendingRow = (
      await canvas.findByText(sessionsFixture.sessions[0]?.session_id ?? "")
    ).closest("tr");
    await expect(pendingRow).toHaveAttribute("data-close-state", "pending");
    await expect(await canvas.findByRole("button", { name: /^Undo$/i })).toBeVisible();
  },
};

export const BatchSelectionFlow: Story = {
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const firstCheckbox = await canvas.findByRole("checkbox", {
      name: /Select session sess-A7c2Kp9LmQ4RsT1v/i,
    });
    const secondCheckbox = await canvas.findByRole("checkbox", {
      name: /Select session sess-Q8n3Va1Zx5Mw2Lp7/i,
    });
    firstCheckbox.focus();
    await userEvent.keyboard("[Space]");
    secondCheckbox.focus();
    await userEvent.keyboard("[Space]");

    await expect(await canvas.findByText(/Selected 2 sessions/i)).toBeVisible();
    await userEvent.click(await canvas.findByRole("button", { name: /^Close selected$/i }));
    await expect((await canvas.findAllByRole("button", { name: /^Undo$/i })).length).toBe(2);
  },
};

export const SwitchingState: Story = {
  args: {
    switchingSessionId: sessionsFixture.sessions[0]?.session_id,
  },
};

export const CopyFormatFlow: Story = {
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("radio", { name: /HTTP URI/i }));
    await expect(await canvas.findByRole("radio", { name: /HTTP URI/i })).toHaveAttribute(
      "data-state",
      "on",
    );
    await expect(canvas.queryByText("Will copy")).not.toBeInTheDocument();
    await expect(
      canvas.queryByText("Preview appears after the first session opens."),
    ).not.toBeInTheDocument();
    await expect(
      await canvas.findByRole("button", {
        name: new RegExp(
          `Copy proxy address for ${sessionsFixture.sessions[0]?.session_id ?? ""}`,
          "i",
        ),
      }),
    ).toBeVisible();
  },
};

export const CreateDialogFlow: Story = {
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: /^Create session$/i }));
    const dialog = within(canvasElement.ownerDocument.body);
    await expect(await dialog.findByRole("dialog", { name: /Create session/i })).toHaveAttribute(
      "data-state",
      "open",
    );
    await expect(await dialog.findByRole("tab", { name: /Single session/i })).toHaveAttribute(
      "data-state",
      "active",
    );
  },
};

export const SwitchDialogFlow: Story = {
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(
      await canvas.findByRole("button", {
        name: new RegExp(`Edit proxy for ${sessionsFixture.sessions[0]?.session_id ?? ""}`, "i"),
      }),
    );
    const dialog = within(canvasElement.ownerDocument.body);
    await dialog.findByRole("dialog", { name: /Switch session proxy/i });
    await dialog.findByRole("combobox", { name: /Sort by/i });
    await dialog.findByRole("button", { name: /Use selected node/i });
  },
};
