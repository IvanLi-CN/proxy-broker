import { CheckCircle2Icon, SirenIcon, TriangleAlertIcon } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { cn } from "@/lib/utils";

interface ActionResponsePanelProps {
  title: string;
  tone?: "success" | "warning" | "error";
  description: string;
  bullets?: string[];
}

export function ActionResponsePanel({
  title,
  tone = "success",
  description,
  bullets,
}: ActionResponsePanelProps) {
  const Icon =
    tone === "error" ? SirenIcon : tone === "warning" ? TriangleAlertIcon : CheckCircle2Icon;

  return (
    <Alert
      variant={tone === "error" ? "destructive" : "default"}
      aria-live={tone === "error" ? "assertive" : "polite"}
      className={cn(
        "rounded-[24px] border px-4 py-4 shadow-sm",
        tone === "success" &&
          "border-emerald-500/20 bg-emerald-500/[0.08] text-emerald-950 dark:text-emerald-50",
        tone === "warning" &&
          "border-amber-500/25 bg-amber-500/[0.1] text-amber-950 dark:text-amber-50",
      )}
    >
      <div className="flex size-9 items-center justify-center rounded-full border border-current/12 bg-background/70">
        <Icon className="size-4" />
      </div>
      <AlertTitle className="text-sm font-semibold tracking-tight">{title}</AlertTitle>
      <AlertDescription className="space-y-3 text-sm leading-6">
        <p>{description}</p>
        {bullets && bullets.length > 0 ? (
          <ul className="grid gap-2 pl-1 text-xs md:text-sm">
            {bullets.map((bullet) => (
              <li key={bullet} className="flex gap-2">
                <span className="mt-2 size-1.5 shrink-0 rounded-full bg-current/70" />
                <span>{bullet}</span>
              </li>
            ))}
          </ul>
        ) : null}
      </AlertDescription>
    </Alert>
  );
}
