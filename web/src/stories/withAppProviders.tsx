import type { Decorator } from "@storybook/react-vite";
import { QueryClientProvider } from "@tanstack/react-query";
import { ThemeProvider } from "next-themes";
import { MemoryRouter } from "react-router-dom";

import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { I18nProvider, type Locale } from "@/i18n";
import { createQueryClient } from "@/lib/query-client";

export const withAppProviders: Decorator = (Story, context) => {
  const queryClient = createQueryClient();
  const theme = context.globals.theme === "dark" ? "dark" : "light";
  const locale = context.globals.locale === "zh-CN" ? ("zh-CN" as Locale) : ("en-US" as Locale);
  const initialEntries =
    Array.isArray(context.parameters.initialEntries) && context.parameters.initialEntries.length > 0
      ? context.parameters.initialEntries
      : ["/"];
  document.documentElement.classList.toggle("dark", theme === "dark");
  document.documentElement.style.colorScheme = theme;

  return (
    <I18nProvider initialLocale={locale}>
      <ThemeProvider attribute="class" forcedTheme={theme}>
        <QueryClientProvider client={queryClient}>
          <TooltipProvider delayDuration={0}>
            <MemoryRouter initialEntries={initialEntries}>
              <Story />
            </MemoryRouter>
            <Toaster richColors position="top-right" />
          </TooltipProvider>
        </QueryClientProvider>
      </ThemeProvider>
    </I18nProvider>
  );
};
