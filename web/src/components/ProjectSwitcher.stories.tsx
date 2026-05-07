import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { userEvent, within } from "storybook/test";

import { ProjectSwitcher } from "@/components/ProjectSwitcher";

const meta = {
  title: "Components/ProjectSwitcher",
  component: ProjectSwitcher,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Sidebar project selector that scopes all API calls to the active project identifier.",
      },
    },
  },
  render: (args) => {
    const [projectId, setProjectId] = useState(args.projectId);
    return (
      <div className="max-w-sm">
        <ProjectSwitcher
          {...args}
          projectId={projectId}
          onProjectIdChange={setProjectId}
          onCreateProject={async (value) => {
            setProjectId(value);
            return value;
          }}
        />
      </div>
    );
  },
  args: {
    projectId: "default",
    projects: ["default", "edge-jp", "lab-us"],
    isLoading: false,
    isCreating: false,
    loadError: null,
    onProjectIdChange: () => undefined,
    onCreateProject: async (value: string) => value,
    onRetryProjects: () => undefined,
  },
} satisfies Meta<typeof ProjectSwitcher>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Populated: Story = {};

export const SearchNoMatch: Story = {
  args: {
    projects: ["default"],
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const overlay = within(canvasElement.ownerDocument.body);
    await userEvent.click(canvas.getByRole("combobox"));
    await userEvent.type(
      await overlay.findByPlaceholderText("Search projects or type a new ID"),
      "tokyo",
    );
  },
};

export const Creating: Story = {
  args: {
    isCreating: true,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const overlay = within(canvasElement.ownerDocument.body);
    await userEvent.click(canvas.getByRole("combobox"));
    await userEvent.type(
      await overlay.findByPlaceholderText("Search projects or type a new ID"),
      "fresh-lab",
    );
  },
};
