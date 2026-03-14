import type * as React from "react";

import { cn } from "@/lib/utils";

type TextareaSize = "sm" | "default" | "lg";

function Textarea({
  className,
  size = "default",
  ...props
}: React.ComponentProps<"textarea"> & {
  size?: TextareaSize;
}) {
  return (
    <textarea
      data-slot="textarea"
      data-size={size}
      className={cn(
        "flex field-sizing-content w-full rounded-lg border border-input bg-transparent text-base transition-colors outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:bg-input/50 disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 md:text-sm dark:bg-input/30 dark:disabled:bg-input/80 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40 data-[size=sm]:min-h-20 data-[size=sm]:px-2 data-[size=sm]:py-1.5 data-[size=default]:min-h-24 data-[size=default]:px-2.5 data-[size=default]:py-2 data-[size=lg]:min-h-28 data-[size=lg]:px-3 data-[size=lg]:py-2.5",
        className,
      )}
      {...props}
    />
  );
}

export { Textarea };
