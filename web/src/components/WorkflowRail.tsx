import { ArrowRightIcon } from "lucide-react";

interface WorkflowStep {
  title: string;
  description: string;
}

interface WorkflowRailProps {
  eyebrow: string;
  title: string;
  steps: WorkflowStep[];
}

export function WorkflowRail({ eyebrow, title, steps }: WorkflowRailProps) {
  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <div className="text-[11px] font-semibold uppercase tracking-[0.32em] text-primary/80">
          {eyebrow}
        </div>
        <div className="text-lg font-semibold tracking-tight text-foreground">{title}</div>
      </div>
      <div className="space-y-3">
        {steps.map((step, index) => (
          <div
            key={step.title}
            className="rounded-2xl border border-border/70 bg-background/80 px-4 py-3 shadow-sm"
          >
            <div className="flex items-start gap-3">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-full border border-primary/20 bg-primary/10 font-mono text-xs font-semibold text-primary">
                {index + 1}
              </div>
              <div className="min-w-0 flex-1 space-y-1">
                <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                  <span>{step.title}</span>
                  {index < steps.length - 1 ? (
                    <ArrowRightIcon className="size-3.5 text-muted-foreground" />
                  ) : null}
                </div>
                <p className="text-sm leading-6 text-muted-foreground">{step.description}</p>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
