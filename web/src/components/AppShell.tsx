import { ActivityIcon, GlobeIcon, LayoutDashboardIcon, RouteIcon } from "lucide-react";
import type { ReactNode } from "react";
import { NavLink } from "react-router-dom";
import { ProfileSwitcher } from "@/components/ProfileSwitcher";
import { ThemeToggle } from "@/components/ThemeToggle";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarSeparator,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { cn } from "@/lib/utils";

interface AppShellProps {
  profileId: string;
  onProfileIdChange: (value: string) => void;
  healthStatus: string;
  children?: ReactNode;
}

const navItems = [
  { to: "/", label: "Overview", icon: LayoutDashboardIcon },
  { to: "/ips", label: "IP Extract", icon: GlobeIcon },
  { to: "/sessions", label: "Sessions", icon: RouteIcon },
];

export function AppShell({ profileId, onProfileIdChange, healthStatus, children }: AppShellProps) {
  return (
    <SidebarProvider>
      <Sidebar collapsible="icon" variant="floating">
        <SidebarHeader className="gap-4 border-b border-sidebar-border/80 px-3 py-4">
          <div className="space-y-1">
            <div className="text-xs font-semibold uppercase tracking-[0.3em] text-sidebar-foreground/65">
              proxy-broker
            </div>
            <div className="text-lg font-semibold text-sidebar-foreground">Operations Console</div>
            <div className="text-sm text-sidebar-foreground/70">
              Tune subscriptions, probe IP pools, and open mihomo sessions.
            </div>
          </div>
          <ProfileSwitcher profileId={profileId} onProfileIdChange={onProfileIdChange} />
        </SidebarHeader>
        <SidebarContent className="px-2 py-4">
          <SidebarGroup>
            <SidebarGroupLabel>Workspace</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {navItems.map((item) => (
                  <SidebarMenuItem key={item.to}>
                    <SidebarMenuButton asChild tooltip={item.label}>
                      <NavLink
                        to={item.to}
                        className={({ isActive }) =>
                          cn(isActive && "bg-sidebar-accent text-sidebar-accent-foreground")
                        }
                        end={item.to === "/"}
                      >
                        <item.icon />
                        <span>{item.label}</span>
                      </NavLink>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
          <SidebarSeparator />
          <SidebarGroup>
            <SidebarGroupLabel>Service health</SidebarGroupLabel>
            <SidebarGroupContent>
              <div className="rounded-xl border border-sidebar-border/80 bg-sidebar-accent/30 p-3 text-sm text-sidebar-foreground">
                <div className="flex items-center gap-2">
                  <ActivityIcon className="size-4 text-sidebar-primary" />
                  <span className="font-medium">Local API</span>
                </div>
                <div className="mt-3 flex items-center gap-2">
                  <Badge variant="secondary" className="bg-sidebar text-sidebar-foreground">
                    {healthStatus.toUpperCase()}
                  </Badge>
                  <span className="text-xs text-sidebar-foreground/65">Polled from /healthz</span>
                </div>
              </div>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarContent>
        <SidebarFooter className="gap-3 border-t border-sidebar-border/80 px-3 py-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <div className="text-sm font-medium text-sidebar-foreground">UI theme</div>
              <div className="text-xs text-sidebar-foreground/60">Optimized for operators.</div>
            </div>
            <ThemeToggle />
          </div>
          <Button
            asChild
            variant="outline"
            className="justify-start border-sidebar-border bg-sidebar-accent/30 text-sidebar-foreground hover:bg-sidebar-accent"
          >
            <a href="https://ui.shadcn.com" rel="noreferrer" target="_blank">
              Inspect design system
            </a>
          </Button>
        </SidebarFooter>
      </Sidebar>
      <SidebarInset>
        <div className="sticky top-0 z-20 flex items-center justify-between gap-4 border-b border-border/70 bg-background/85 px-4 py-3 backdrop-blur-sm md:px-6">
          <div className="flex items-center gap-3">
            <SidebarTrigger />
            <div>
              <div className="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">
                Local operator plane
              </div>
              <div className="text-sm font-medium text-foreground md:text-base">
                Profile <span className="font-mono text-primary">{profileId}</span>
              </div>
            </div>
          </div>
          <Badge variant="outline" className="gap-2 rounded-full px-3 py-1 font-mono text-xs">
            <span className="size-2 rounded-full bg-emerald-500" />
            127.0.0.1
          </Badge>
        </div>
        <div className="mx-auto flex w-full max-w-7xl flex-1 flex-col gap-6 px-4 py-6 md:px-6">
          {children}
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
