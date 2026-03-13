import type { Meta, StoryObj } from "@storybook/react-vite";
import { HomeIcon, RouterIcon } from "lucide-react";

import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";

const meta = {
  title: "UI/Sidebar",
  component: Sidebar,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component: "Adaptive navigation shell used by the operator console layout.",
      },
    },
  },
} satisfies Meta<typeof Sidebar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <SidebarProvider>
      <Sidebar collapsible="icon" variant="floating">
        <SidebarHeader className="border-b border-sidebar-border/80 px-3 py-4">
          <div className="text-sm font-semibold text-sidebar-foreground">Sidebar demo</div>
        </SidebarHeader>
        <SidebarContent className="px-2 py-4">
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton>
                <HomeIcon />
                <span>Overview</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
            <SidebarMenuItem>
              <SidebarMenuButton>
                <RouterIcon />
                <span>Sessions</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarContent>
      </Sidebar>
      <SidebarInset>
        <div className="p-6">
          <SidebarTrigger />
        </div>
      </SidebarInset>
    </SidebarProvider>
  ),
};
