import { Settings2Icon } from "lucide-react";

import { ActionResponsePanel } from "@/components/ActionResponsePanel";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { useI18n } from "@/i18n";
import { cn } from "@/lib/utils";

interface ProjectProxyPolicyCardProps {
  projectId: string;
  useGlobalProxies: boolean;
  proxySettingsLoading: boolean;
  updatingSettings: boolean;
  proxySettingsError?: string | null;
  onToggleUseGlobalProxies: (nextValue: boolean) => void | Promise<void>;
}

export function ProjectProxyPolicyCard({
  projectId,
  useGlobalProxies,
  proxySettingsLoading,
  updatingSettings,
  proxySettingsError,
  onToggleUseGlobalProxies,
}: ProjectProxyPolicyCardProps) {
  const { t } = useI18n();

  return (
    <Card className="overflow-hidden border-border/70 bg-card/96 shadow-[0_20px_60px_-40px_rgba(15,23,42,0.5)]">
      <CardHeader className="gap-3 border-b border-border/70 bg-muted/15 pb-4">
        <div className="space-y-1.5">
          <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
            {t("Project policy")}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <CardTitle className="flex items-center gap-2 text-lg tracking-tight">
              <Settings2Icon className="size-4 text-primary" />
              {t("Use global pool for {projectId}", { projectId })}
            </CardTitle>
            <Badge
              variant={useGlobalProxies ? "default" : "secondary"}
              className={cn(
                "rounded-full px-2 py-0.5 font-mono text-[10px] uppercase tracking-[0.16em]",
                !useGlobalProxies && "bg-muted text-foreground",
              )}
            >
              {useGlobalProxies ? t("global enabled") : t("local-only")}
            </Badge>
          </div>
          <CardDescription className="text-sm leading-5 text-muted-foreground">
            {t("Only changes whether {projectId} inherits the global pool.", {
              projectId,
            })}
          </CardDescription>
        </div>
      </CardHeader>
      <CardContent className="space-y-3 pt-4">
        <div className="rounded-[16px] border border-border/70 bg-background/80 p-3">
          <div className="flex items-center gap-3">
            <Checkbox
              id="use-global-proxies"
              checked={useGlobalProxies}
              disabled={proxySettingsLoading || updatingSettings}
              onCheckedChange={(checked) => {
                void onToggleUseGlobalProxies(checked === true);
              }}
              aria-label={t("Use global pool for {projectId}", { projectId })}
            />
            <Label
              htmlFor="use-global-proxies"
              className="cursor-pointer text-sm font-medium text-foreground"
            >
              {t("Compose {projectId} from the global pool as well", { projectId })}
            </Label>
          </div>
        </div>

        <p className="text-xs leading-5 text-muted-foreground">
          {t(
            "Turning this off immediately rebuilds the project from local nodes only and removes sessions that depended on global-only nodes.",
          )}
        </p>

        {proxySettingsError ? (
          <ActionResponsePanel
            title={t("Project proxy settings unavailable")}
            description={proxySettingsError}
            tone="error"
          />
        ) : null}
      </CardContent>
    </Card>
  );
}
