import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "@storybook/test";

import { OpenSessionForm } from "@/features/sessions/components/OpenSessionForm";
import { sessionFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Sessions/OpenSessionForm",
  component: OpenSessionForm,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Single-session opener with direct IP pinning and selector-based fallbacks.",
      },
    },
  },
  args: {
    isPending: false,
    onSubmit: fn(),
    response: sessionFixture,
    error: null,
  },
} satisfies Meta<typeof OpenSessionForm>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ErrorState: Story = {
  args: {
    response: null,
    error: "ip_not_found: selector returned no IPs",
  },
};

export const Interaction: Story = {
  args: {
    response: null,
    error: null,
    onSubmit: fn(),
  },
  async play({ canvasElement, args }) {
    const canvas = within(canvasElement);
    await userEvent.clear(canvas.getByLabelText("Desired port"));
    await userEvent.type(canvas.getByLabelText("Desired port"), "10088");
    await userEvent.click(canvas.getByRole("button", { name: /open session/i }));
    expect(args.onSubmit).toHaveBeenCalled();
  },
};
