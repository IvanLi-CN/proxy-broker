import type { Meta, StoryObj } from "@storybook/react-vite";
import { CircleAlertIcon } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

const meta = {
  title: "UI/Alert",
  component: Alert,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Inline status surface for success, warning, and error summaries.",
      },
    },
  },
} satisfies Meta<typeof Alert>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <Alert>
      <CircleAlertIcon className="size-4" />
      <AlertTitle>Backend warning</AlertTitle>
      <AlertDescription>One proxy reused a cached IP after DNS resolution failed.</AlertDescription>
    </Alert>
  ),
};
