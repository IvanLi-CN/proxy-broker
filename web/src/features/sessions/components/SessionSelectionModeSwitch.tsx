import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useI18n } from "@/i18n";
import type { SessionSelectionMode } from "@/lib/types";
import { cn } from "@/lib/utils";

interface SessionSelectionModeSwitchProps {
  value: SessionSelectionMode;
  onChange: (value: SessionSelectionMode) => void;
  size?: "default" | "sm";
  className?: string;
}

export function SessionSelectionModeSwitch({
  value,
  onChange,
  size = "default",
  className,
}: SessionSelectionModeSwitchProps) {
  const { t } = useI18n();
  const sessionSelectionModeOptions: Array<{
    value: SessionSelectionMode;
    title: string;
    compactTitle: string;
    description: string;
  }> = [
    {
      value: "any",
      title: t("Any"),
      compactTitle: t("Any"),
      description: t("Take the first candidate from the current pool."),
    },
    {
      value: "geo",
      title: t("Country / region"),
      compactTitle: t("Region"),
      description: t("Narrow by country or city, then take the first surviving candidate."),
    },
    {
      value: "ip",
      title: "IP",
      compactTitle: "IP",
      description: t("Target one or more IPs directly, then pick the first match."),
    },
  ];

  return (
    <Tabs
      value={value}
      onValueChange={(nextValue) => onChange(nextValue as SessionSelectionMode)}
      className={cn("w-full gap-0", className)}
    >
      <TabsList
        aria-label={t("Targeting mode")}
        className={cn(
          "inline-grid w-full grid-cols-3 rounded-full border border-slate-200 bg-slate-100/90 p-0.5 text-slate-500 shadow-[inset_0_1px_0_rgba(255,255,255,0.85)]",
          size === "sm" ? "h-9 min-w-0" : "h-10 min-w-0 max-w-[396px]",
        )}
      >
        {sessionSelectionModeOptions.map((option) => (
          <TabsTrigger
            key={option.value}
            value={option.value}
            className={cn(
              "min-w-0 rounded-full border border-transparent font-semibold text-slate-500 transition-[background-color,color,box-shadow,border-color] duration-200 hover:text-slate-700 data-active:border-slate-200 data-active:bg-white data-active:text-primary data-active:shadow-[0_1px_2px_rgba(15,23,42,0.08)] after:hidden",
              size === "sm" ? "px-2 py-1 text-[13px] sm:px-3" : "px-4 py-1.5 text-sm",
            )}
          >
            <span className="truncate">{size === "sm" ? option.compactTitle : option.title}</span>
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
}
