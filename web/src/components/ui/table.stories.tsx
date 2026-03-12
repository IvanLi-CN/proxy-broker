import type { Meta, StoryObj } from "@storybook/react-vite";

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

const meta = {
  title: "UI/Table",
  component: Table,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Structured data primitive used for IP lists and active sessions.",
      },
    },
  },
} satisfies Meta<typeof Table>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <div className="rounded-2xl border border-border/70 bg-card/90 p-2">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Port</TableHead>
            <TableHead>Proxy</TableHead>
            <TableHead>IP</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow>
            <TableCell>10080</TableCell>
            <TableCell>JP-Tokyo-Entry</TableCell>
            <TableCell>203.0.113.10</TableCell>
          </TableRow>
          <TableRow>
            <TableCell>10081</TableCell>
            <TableCell>JP-Osaka-Edge</TableCell>
            <TableCell>203.0.113.88</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </div>
  ),
};
