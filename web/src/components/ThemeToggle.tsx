import { MoonStarIcon, SunMediumIcon } from "lucide-react";
import { useTheme } from "next-themes";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useI18n } from "@/i18n";

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const { t } = useI18n();
  const isDark = theme === "dark";

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="outline"
          size="icon-sm"
          aria-label={t("Toggle theme")}
          className="border-sidebar-border bg-background/70 hover:bg-background"
          onClick={() => setTheme(isDark ? "light" : "dark")}
        >
          {isDark ? <SunMediumIcon /> : <MoonStarIcon />}
        </Button>
      </TooltipTrigger>
      <TooltipContent>
        {isDark ? t("Switch to light mode") : t("Switch to dark mode")}
      </TooltipContent>
    </Tooltip>
  );
}
