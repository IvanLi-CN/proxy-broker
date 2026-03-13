import type { Meta, StoryObj } from "@storybook/react-vite";

import { NotFoundPage } from "@/pages/NotFoundPage";

const meta = {
  title: "Pages/NotFoundPage",
  component: NotFoundPage,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Fallback route for unknown paths inside the operator console.",
      },
    },
  },
} satisfies Meta<typeof NotFoundPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
