import type { Meta, StoryObj } from "@storybook/react-vite";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

const meta = {
  title: "UI/Card",
  component: Card,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Content container used for panels, forms, and summary slices.",
      },
    },
  },
} satisfies Meta<typeof Card>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <Card className="max-w-md">
      <CardHeader>
        <CardTitle>Route candidate summary</CardTitle>
        <CardDescription>Use cards to group one operational action at a time.</CardDescription>
        <CardAction>
          <Button size="sm" variant="ghost">
            Inspect
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>26 reachable IPs are ready for session selection in this profile.</CardContent>
      <CardFooter>Updated 2 minutes ago</CardFooter>
    </Card>
  ),
};
