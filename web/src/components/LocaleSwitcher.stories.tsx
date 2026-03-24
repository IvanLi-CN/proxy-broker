import type { Meta, StoryObj } from "@storybook/react-vite";

import { LocaleSwitcher } from "@/components/LocaleSwitcher";

const meta = {
  title: "Components/LocaleSwitcher",
  component: LocaleSwitcher,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Locale picker used in the app shell footer so operators can switch between English and Simplified Chinese without reloading the page.",
      },
    },
  },
} satisfies Meta<typeof LocaleSwitcher>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ZhCN: Story = {
  globals: {
    locale: "zh-CN",
  },
};
