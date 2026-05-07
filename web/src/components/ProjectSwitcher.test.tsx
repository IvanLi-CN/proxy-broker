import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { ProjectSwitcher } from "@/components/ProjectSwitcher";

describe("ProjectSwitcher", () => {
  it("filters and selects an existing project", async () => {
    const user = userEvent.setup();
    const onProjectIdChange = vi.fn();

    render(
      <ProjectSwitcher
        projectId="default"
        projects={["default", "edge-jp", "lab-us"]}
        onProjectIdChange={onProjectIdChange}
        onCreateProject={async (value) => value}
      />,
    );

    await user.click(screen.getByRole("combobox", { name: /project id/i }));
    await user.type(screen.getByPlaceholderText("Search projects or type a new ID"), "jp");
    await user.click(screen.getByText("edge-jp"));

    expect(onProjectIdChange).toHaveBeenCalledWith("edge-jp");
  });

  it("creates a new project from the current query", async () => {
    const user = userEvent.setup();
    const onCreateProject = vi.fn(async (value: string) => value);

    render(
      <ProjectSwitcher
        projectId="default"
        projects={["default"]}
        onProjectIdChange={() => undefined}
        onCreateProject={onCreateProject}
      />,
    );

    await user.click(screen.getByRole("combobox", { name: /project id/i }));
    await user.type(screen.getByPlaceholderText("Search projects or type a new ID"), "fresh-lab");
    await user.click(screen.getByText('Create "fresh-lab"'));

    await waitFor(() => {
      expect(onCreateProject).toHaveBeenCalledWith("fresh-lab");
    });
  });

  it("offers a global project option before concrete projects", async () => {
    const user = userEvent.setup();
    const onProjectIdChange = vi.fn();

    render(
      <ProjectSwitcher
        projectId="default"
        projects={["default", "edge-jp"]}
        onProjectIdChange={onProjectIdChange}
        onCreateProject={async (value) => value}
      />,
    );

    await user.click(screen.getByRole("combobox", { name: /project id/i }));
    await user.click(screen.getByText("Global"));

    expect(onProjectIdChange).toHaveBeenCalledWith("__global__");
  });
});
