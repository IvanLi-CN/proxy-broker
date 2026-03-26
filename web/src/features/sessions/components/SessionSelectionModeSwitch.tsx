import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { SessionSelectionMode } from "@/lib/types";
import { cn } from "@/lib/utils";

export const sessionSelectionModeOptions: Array<{
  value: SessionSelectionMode;
  title: string;
  description: string;
}> = [
  {
    value: "any",
    title: "不限",
    description: "从当前候选集合里直接按提取顺序拿第 1 个。",
  },
  {
    value: "geo",
    title: "国家/地区",
    description: "先按国家或城市收窄，再按提取顺序拿第 1 个。",
  },
  {
    value: "ip",
    title: "IP",
    description: "手动圈定一个或多个 IP，再按提取顺序命中。",
  },
];

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
  return (
    <Tabs
      value={value}
      onValueChange={(nextValue) => onChange(nextValue as SessionSelectionMode)}
      className={cn("w-full gap-0", className)}
    >
      <TabsList
        aria-label="选择范围"
        className={cn(
          "inline-grid w-full grid-cols-3 rounded-full border border-slate-200 bg-slate-100/90 p-0.5 text-slate-500 shadow-[inset_0_1px_0_rgba(255,255,255,0.85)]",
          size === "sm" ? "h-9 max-w-[360px]" : "h-10 max-w-[396px]",
        )}
      >
        {sessionSelectionModeOptions.map((option) => (
          <TabsTrigger
            key={option.value}
            value={option.value}
            className={cn(
              "rounded-full border border-transparent font-semibold text-slate-500 transition-[background-color,color,box-shadow,border-color] duration-200 hover:text-slate-700 data-active:border-slate-200 data-active:bg-white data-active:text-primary data-active:shadow-[0_1px_2px_rgba(15,23,42,0.08)] after:hidden",
              size === "sm" ? "px-3 py-1 text-[13px]" : "px-4 py-1.5 text-sm",
            )}
          >
            {option.title}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
}
