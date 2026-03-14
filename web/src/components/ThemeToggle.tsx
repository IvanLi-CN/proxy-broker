import { MoonStarIcon, SunMediumIcon } from "lucide-react";
import { useTheme } from "next-themes";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const isDark = theme === "dark";

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="outline"
          size="icon-sm"
          aria-label="Toggle theme"
          className="border-sidebar-border bg-background/70 hover:bg-background"
          onClick={() => setTheme(isDark ? "light" : "dark")}
        >
          {isDark ? <SunMediumIcon /> : <MoonStarIcon />}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{isDark ? "Switch to light mode" : "Switch to dark mode"}</TooltipContent>
    </Tooltip>
  );
}
