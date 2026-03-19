import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { ProfileSwitcher } from "@/components/ProfileSwitcher";

describe("ProfileSwitcher", () => {
  it("filters and selects an existing profile", async () => {
    const user = userEvent.setup();
    const onProfileIdChange = vi.fn();

    render(
      <ProfileSwitcher
        profileId="default"
        profiles={["default", "edge-jp", "lab-us"]}
        onProfileIdChange={onProfileIdChange}
        onCreateProfile={async (value) => value}
      />,
    );

    await user.click(screen.getByRole("combobox", { name: /profile id/i }));
    await user.type(screen.getByPlaceholderText("Search profiles or type a new ID"), "jp");
    await user.click(screen.getByText("edge-jp"));

    expect(onProfileIdChange).toHaveBeenCalledWith("edge-jp");
  });

  it("creates a new profile from the current query", async () => {
    const user = userEvent.setup();
    const onCreateProfile = vi.fn(async (value: string) => value);

    render(
      <ProfileSwitcher
        profileId="default"
        profiles={["default"]}
        onProfileIdChange={() => undefined}
        onCreateProfile={onCreateProfile}
      />,
    );

    await user.click(screen.getByRole("combobox", { name: /profile id/i }));
    await user.type(screen.getByPlaceholderText("Search profiles or type a new ID"), "fresh-lab");
    await user.click(screen.getByText('Create "fresh-lab"'));

    await waitFor(() => {
      expect(onCreateProfile).toHaveBeenCalledWith("fresh-lab");
    });
  });
});
