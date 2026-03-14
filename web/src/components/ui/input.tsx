import type * as React from "react";

import { cn } from "@/lib/utils";

type InputSize = "sm" | "default" | "lg";

function Input({
  className,
  type,
  size = "default",
  ...props
}: Omit<React.ComponentProps<"input">, "size"> & {
  size?: InputSize;
}) {
  return (
    <input
      type={type}
      data-slot="input"
      data-size={size}
      className={cn(
        "w-full min-w-0 rounded-lg border border-input bg-transparent text-base transition-colors outline-none file:inline-flex file:h-6 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:cursor-not-allowed disabled:bg-input/50 disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 md:text-sm dark:bg-input/30 dark:disabled:bg-input/80 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40 data-[size=sm]:h-7 data-[size=sm]:px-2 data-[size=sm]:py-1 data-[size=default]:h-8 data-[size=default]:px-2.5 data-[size=default]:py-1 data-[size=lg]:h-11 data-[size=lg]:px-3 data-[size=lg]:py-2",
        className,
      )}
      {...props}
    />
  );
}

export { Input };
