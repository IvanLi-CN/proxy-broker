import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { SessionCreateDialog } from "@/features/sessions/components/SessionCreateDialog";

const meta = {
  title: "Features/Sessions/SessionCreateDialog",
  component: SessionCreateDialog,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Dialog-only entry point for creating single or batch sessions without occupying the main session list layout.",
      },
    },
  },
  args: {
    open: true,
    onOpenChange: fn(),
    openError: null,
    batchError: null,
    openResponse: null,
    batchResponse: null,
    opening: false,
    batchOpening: false,
    suggestedPort: 10080,
    onOpenSession: fn(),
    onOpenBatch: fn(),
    searchSessionOptions: fn(async () => []),
  },
} satisfies Meta<typeof SessionCreateDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};

export const CompactViewport: Story = {
  args: {},
  parameters: {
    viewport: {
      defaultViewport: "mobile2",
    },
  },
};

export const BatchError: Story = {
  args: {
    batchError: "The backend rejected one row, so the batch rolled back.",
  },
};

export const ZhCN: Story = {
  args: {},
  globals: {
    locale: "zh-CN",
  },
};
