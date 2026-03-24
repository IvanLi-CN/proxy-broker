import { QueryClientProvider } from "@tanstack/react-query";
import { ThemeProvider } from "next-themes";
import type { PropsWithChildren } from "react";
import { useState } from "react";

import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { I18nProvider, type Locale } from "@/i18n";
import { createQueryClient } from "@/lib/query-client";

export function AppProviders({
  children,
  initialLocale,
}: PropsWithChildren<{ initialLocale?: Locale }>) {
  const [queryClient] = useState(createQueryClient);

  return (
    <I18nProvider initialLocale={initialLocale}>
      <ThemeProvider attribute="class" defaultTheme="light" enableSystem={false}>
        <QueryClientProvider client={queryClient}>
          <TooltipProvider delayDuration={150}>
            {children}
            <Toaster richColors position="top-right" />
          </TooltipProvider>
        </QueryClientProvider>
      </ThemeProvider>
    </I18nProvider>
  );
}
