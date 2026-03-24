import { LanguagesIcon } from "lucide-react";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useI18n } from "@/i18n";

const localeLabels = {
  "en-US": "English",
  "zh-CN": "Simplified Chinese",
} as const;

export function LocaleSwitcher() {
  const { locale, setLocale, t } = useI18n();

  return (
    <div className="space-y-2">
      <div>
        <div className="text-sm font-medium text-sidebar-foreground">{t("Language")}</div>
        <div className="text-xs text-sidebar-foreground/60">
          {t("Choose the interface language.")}
        </div>
      </div>
      <Select value={locale} onValueChange={(value) => setLocale(value as typeof locale)}>
        <SelectTrigger
          aria-label={t("Operator console language")}
          className="w-full justify-between rounded-xl border-sidebar-border bg-background/70 text-sidebar-foreground hover:bg-background"
        >
          <div className="flex items-center gap-2">
            <LanguagesIcon className="size-4 text-sidebar-primary" />
            <SelectValue />
          </div>
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="en-US">{t(localeLabels["en-US"])}</SelectItem>
          <SelectItem value="zh-CN">{t(localeLabels["zh-CN"])}</SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
}
