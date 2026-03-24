import {
  ActivityIcon,
  ChevronRightIcon,
  ClipboardListIcon,
  CommandIcon,
  GlobeIcon,
  LayoutDashboardIcon,
  RadioTowerIcon,
  RouteIcon,
} from "lucide-react";
import type { ReactNode } from "react";
import { NavLink } from "react-router-dom";

import { CurrentUserSummary } from "@/components/CurrentUserSummary";
import { LocaleSwitcher } from "@/components/LocaleSwitcher";
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
import { useI18n } from "@/i18n";
import { formatHealthStatus } from "@/lib/format";
import type { CurrentUserState } from "@/lib/types";
import { cn } from "@/lib/utils";

interface AppShellProps {
  profileId: string;
  profiles: string[];
  profilesLoading?: boolean;
  profilesCreating?: boolean;
  profilesError?: string | null;
  onProfileIdChange: (value: string) => void;
  onCreateProfile: (value: string) => Promise<string>;
  onRetryProfiles?: () => void;
  healthStatus: string;
  currentUser: CurrentUserState;
  children?: ReactNode;
}

export function AppShell({
  profileId,
  profiles,
  profilesLoading = false,
  profilesCreating = false,
  profilesError = null,
  onProfileIdChange,
  onCreateProfile,
  onRetryProfiles,
  healthStatus,
  currentUser,
  children,
}: AppShellProps) {
  const { t } = useI18n();
  const isHealthy = (healthStatus ?? "").toLowerCase() === "ok";
  const healthStatusLabel = formatHealthStatus(healthStatus, t);
  const navItems = [
    {
      to: "/",
      label: t("Overview"),
      icon: LayoutDashboardIcon,
      meta: t("Load feeds and refresh pool metadata"),
    },
    {
      to: "/tasks",
      label: t("Tasks"),
      icon: ClipboardListIcon,
      meta: t("Watch scheduled sync and metadata refresh runs"),
    },
    {
      to: "/ips",
      label: t("IP Extract"),
      icon: GlobeIcon,
      meta: t("Filter the pool down to candidate edges"),
    },
    {
      to: "/sessions",
      label: t("Sessions"),
      icon: RouteIcon,
      meta: t("Open, audit, and close live listeners"),
    },
  ];

  return (
    <SidebarProvider>
      <Sidebar collapsible="icon" variant="floating" className="p-3">
        <SidebarHeader className="gap-3 rounded-[26px] border border-sidebar-border/80 bg-sidebar/96 px-3 py-3 shadow-[0_24px_60px_-40px_rgba(15,23,42,0.5)]">
          <div className="rounded-[22px] border border-sidebar-border/80 bg-[linear-gradient(150deg,rgba(255,255,255,0.95),rgba(232,240,255,0.9)_48%,rgba(229,245,245,0.88))] px-3 py-3 text-sidebar-foreground dark:bg-[linear-gradient(150deg,rgba(18,25,38,0.98),rgba(22,31,52,0.94)_48%,rgba(12,44,50,0.9))]">
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <div className="text-[10px] font-semibold uppercase tracking-[0.32em] text-sidebar-foreground/62">
                  {t("Proxy broker")}
                </div>
                <div className="mt-1 text-sm font-semibold tracking-[-0.02em] text-sidebar-foreground">
                  {t("Operator plane")}
                </div>
              </div>
              <div className="rounded-xl border border-sidebar-border/80 bg-background/75 p-2.5">
                <CommandIcon className="size-4 text-sidebar-primary" />
              </div>
            </div>
          </div>
          <ProfileSwitcher
            profileId={profileId}
            profiles={profiles}
            isLoading={profilesLoading}
            isCreating={profilesCreating}
            loadError={profilesError}
            onProfileIdChange={onProfileIdChange}
            onCreateProfile={onCreateProfile}
            onRetryProfiles={onRetryProfiles}
          />
        </SidebarHeader>
        <SidebarContent className="px-2 py-4">
          <SidebarGroup>
            <SidebarGroupLabel>{t("Workspace")}</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {navItems.map((item) => (
                  <SidebarMenuItem key={item.to}>
                    <SidebarMenuButton asChild tooltip={item.label} className="h-auto">
                      <NavLink
                        to={item.to}
                        end={item.to === "/"}
                        className={({ isActive }) =>
                          cn(
                            "group/nav flex items-start gap-3 rounded-2xl border border-transparent px-3 py-3 transition-all duration-200 hover:border-sidebar-border hover:bg-sidebar-accent/65",
                            isActive &&
                              "border-sidebar-border bg-sidebar-accent text-sidebar-accent-foreground shadow-sm",
                          )
                        }
                      >
                        <div className="mt-0.5 rounded-xl border border-sidebar-border/80 bg-background/70 p-2 text-sidebar-primary">
                          <item.icon className="size-4" />
                        </div>
                        <div className="min-w-0 flex-1 space-y-1 group-data-[collapsible=icon]:hidden">
                          <div className="flex items-center gap-2 text-sm font-semibold">
                            <span>{item.label}</span>
                            <ChevronRightIcon className="size-3.5 text-sidebar-foreground/45 transition-transform group-hover/nav:translate-x-0.5" />
                          </div>
                          <div className="text-xs leading-5 text-sidebar-foreground/65">
                            {item.meta}
                          </div>
                        </div>
                      </NavLink>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
          <SidebarSeparator />
          <SidebarGroup>
            <SidebarGroupLabel>{t("Runtime")}</SidebarGroupLabel>
            <SidebarGroupContent>
              <div className="space-y-3 rounded-[24px] border border-sidebar-border/80 bg-sidebar-accent/35 p-3 text-sidebar-foreground">
                <div className="flex items-center gap-2 text-sm font-medium">
                  <ActivityIcon className="size-4 text-sidebar-primary" />
                  {t("Local API heartbeat")}
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Badge
                    variant="secondary"
                    className="rounded-full bg-background/80 font-mono text-[11px] uppercase tracking-[0.18em] text-sidebar-foreground"
                  >
                    {healthStatusLabel}
                  </Badge>
                  <span className="text-xs text-sidebar-foreground/65">
                    {t("Refreshed from /healthz")}
                  </span>
                </div>
                <div className="rounded-2xl border border-sidebar-border/80 bg-background/70 px-3 py-2 text-xs leading-5 text-sidebar-foreground/68">
                  {t("Use cmd/ctrl + b to collapse the sidebar when the tables need more room.")}
                </div>
              </div>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarContent>
        <SidebarFooter className="gap-3 rounded-[28px] border border-sidebar-border/80 bg-sidebar/96 px-3 py-4 shadow-[0_24px_60px_-40px_rgba(15,23,42,0.5)]">
          <div className="space-y-3 rounded-2xl border border-sidebar-border/80 bg-sidebar-accent/35 px-3 py-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <div className="text-sm font-medium text-sidebar-foreground">
                  {t("Theme surface")}
                </div>
                <div className="text-xs text-sidebar-foreground/60">
                  {t("Light-first, still dark-safe.")}
                </div>
              </div>
              <ThemeToggle />
            </div>
            <LocaleSwitcher />
          </div>
          <Button
            asChild
            variant="outline"
            className="justify-start border-sidebar-border bg-background/70 text-sidebar-foreground hover:bg-background"
          >
            <a href="https://ui.shadcn.com" rel="noreferrer" target="_blank">
              <RadioTowerIcon className="size-4" />
              {t("Inspect shadcn/ui system")}
            </a>
          </Button>
        </SidebarFooter>
      </Sidebar>
      <SidebarInset>
        <div className="sticky top-0 z-20 flex flex-wrap items-center justify-between gap-4 border-b border-border/70 bg-background/78 px-5 py-3 backdrop-blur-xl md:px-7">
          <div className="flex items-center gap-3">
            <SidebarTrigger className="border-border/70 bg-background/80 hover:bg-background" />
            <div>
              <div className="text-[11px] font-semibold uppercase tracking-[0.28em] text-muted-foreground">
                {t("Local operator plane")}
              </div>
              <div className="flex items-center gap-1 text-sm font-medium text-foreground md:text-base">
                <span>{t("Profile")}</span>
                <span className="font-mono">{profileId}</span>
              </div>
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <CurrentUserSummary currentUser={currentUser} variant="compact" />
            <Badge
              variant="outline"
              className="rounded-full bg-background/80 px-3 py-1 font-mono text-[11px] uppercase tracking-[0.18em]"
            >
              127.0.0.1
            </Badge>
            <Badge
              variant="outline"
              className={cn(
                "rounded-full px-3 py-1 font-mono text-[11px] uppercase tracking-[0.18em]",
                isHealthy
                  ? "border-emerald-500/20 bg-emerald-500/[0.09] text-emerald-700 dark:text-emerald-300"
                  : "border-amber-500/25 bg-amber-500/[0.12] text-amber-700 dark:text-amber-300",
              )}
            >
              {healthStatusLabel}
            </Badge>
          </div>
        </div>
        <div className="mx-auto flex w-full max-w-[1480px] flex-1 flex-col gap-8 px-5 py-7 md:px-7">
          {children}
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
