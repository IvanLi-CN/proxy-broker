import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "@storybook/test";

import { IpFiltersForm } from "@/features/ips/components/IpFiltersForm";

const meta = {
  title: "Features/IP Extract/IpFiltersForm",
  component: IpFiltersForm,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Filter form that converts CSV/newline inputs into the existing extract request schema.",
      },
    },
  },
  args: {
    isPending: false,
    onSubmit: fn(),
  },
} satisfies Meta<typeof IpFiltersForm>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Interaction: Story = {
  async play({ canvasElement, args }) {
    const canvas = within(canvasElement);
    await userEvent.clear(canvas.getByLabelText("Cities"));
    await userEvent.type(canvas.getByLabelText("Cities"), "Tokyo");
    await userEvent.click(canvas.getByRole("button", { name: /extract ips/i }));
    expect(args.onSubmit).toHaveBeenCalled();
  },
};
