import type { Meta, StoryObj } from "@storybook/react-vite";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

const meta = {
  title: "UI/Tabs",
  component: Tabs,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Segmented navigation primitive used for single/batch session forms.",
      },
    },
  },
} satisfies Meta<typeof Tabs>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <Tabs defaultValue="single" className="max-w-lg">
      <TabsList>
        <TabsTrigger value="single">Single</TabsTrigger>
        <TabsTrigger value="batch">Batch</TabsTrigger>
      </TabsList>
      <TabsContent value="single">Single-session form goes here.</TabsContent>
      <TabsContent value="batch">Batch-request builder goes here.</TabsContent>
    </Tabs>
  ),
};
