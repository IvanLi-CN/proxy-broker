import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "@storybook/test";

import { OpenBatchForm } from "@/features/sessions/components/OpenBatchForm";
import { batchFixture } from "@/mocks/fixtures";

const meta = {
  title: "Features/Sessions/OpenBatchForm",
  component: OpenBatchForm,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Field-array editor for building transactional open-batch requests without hand-editing JSON.",
      },
    },
  },
  args: {
    isPending: false,
    onSubmit: fn(),
    response: batchFixture,
    error: null,
  },
} satisfies Meta<typeof OpenBatchForm>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Interaction: Story = {
  args: {
    response: null,
    error: null,
    onSubmit: fn(),
  },
  async play({ canvasElement, args }) {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: /add request row/i }));
    await userEvent.click(canvas.getByRole("button", { name: /open batch/i }));
    expect(args.onSubmit).toHaveBeenCalled();
  },
};
