import type { PropsWithChildren } from "react";
import { createContext, useContext, useEffect, useMemo, useState } from "react";

import { enUS } from "@/i18n/messages/en-US";
import { zhCN } from "@/i18n/messages/zh-CN";
import {
  type Locale,
  type MessageCatalog,
  supportedLocales,
  type TranslationValues,
  type Translator,
} from "@/i18n/types";

const LOCALE_STORAGE_KEY = "proxy-broker.locale";
const DEFAULT_LOCALE: Locale = "en-US";
const CHINESE_LOCALE: Locale = "zh-CN";
const DEFAULT_DOCUMENT_LANGUAGE = "en";
const CHINESE_DOCUMENT_LANGUAGE = "zh-Hans";

function normalizeLocale(value?: string | null): Locale | null {
  if (!value) {
    return null;
  }

  const normalized = value.trim().toLowerCase();
  if (!normalized) {
    return null;
  }
  if (normalized.startsWith("zh")) {
    return CHINESE_LOCALE;
  }
  if (normalized.startsWith("en")) {
    return DEFAULT_LOCALE;
  }
  return null;
}

function resolveLocale(candidates: Array<string | null | undefined>): Locale {
  for (const candidate of candidates) {
    const locale = normalizeLocale(candidate);
    if (locale) {
      return locale;
    }
  }
  return DEFAULT_LOCALE;
}

function resolveInitialLocale() {
  if (typeof window === "undefined") {
    return DEFAULT_LOCALE;
  }

  const navigatorLocales = Array.isArray(window.navigator.languages)
    ? window.navigator.languages
    : [window.navigator.language];
  return resolveLocale([window.localStorage.getItem(LOCALE_STORAGE_KEY), ...navigatorLocales]);
}

function interpolate(message: string, values?: TranslationValues) {
  if (!values) {
    return message;
  }

  return message.replace(/\{(\w+)\}/g, (_, key: string) => {
    const value = values[key];
    return value == null ? `{${key}}` : String(value);
  });
}

function getCatalog(locale: Locale): MessageCatalog {
  return locale === CHINESE_LOCALE ? zhCN : enUS;
}

function localeToDocumentLanguage(locale: Locale) {
  return locale === CHINESE_LOCALE ? CHINESE_DOCUMENT_LANGUAGE : DEFAULT_DOCUMENT_LANGUAGE;
}

function createTranslator(locale: Locale): Translator {
  const catalog = getCatalog(locale);
  return (message, values) => interpolate(catalog[message] ?? message, values);
}

export interface I18nContextValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: Translator;
  formatDateTime: (value?: number | null) => string;
  formatNumber: (value: number) => string;
}

const defaultContext: I18nContextValue = {
  locale: DEFAULT_LOCALE,
  setLocale: () => undefined,
  t: createTranslator(DEFAULT_LOCALE),
  formatDateTime: (value) => {
    if (!value) {
      return "Never";
    }
    return new Intl.DateTimeFormat(DEFAULT_LOCALE, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(value * 1000);
  },
  formatNumber: (value) => new Intl.NumberFormat(DEFAULT_LOCALE).format(value),
};

const I18nContext = createContext<I18nContextValue>(defaultContext);

export function I18nProvider({
  children,
  initialLocale,
}: PropsWithChildren<{ initialLocale?: Locale }>) {
  const [locale, setLocale] = useState<Locale>(() => initialLocale ?? resolveInitialLocale());

  useEffect(() => {
    if (initialLocale) {
      setLocale(initialLocale);
    }
  }, [initialLocale]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    window.localStorage.setItem(LOCALE_STORAGE_KEY, locale);
    document.documentElement.lang = localeToDocumentLanguage(locale);
  }, [locale]);

  const value = useMemo<I18nContextValue>(() => {
    const t = createTranslator(locale);
    return {
      locale,
      setLocale,
      t,
      formatDateTime: (value) => {
        if (!value) {
          return t("Never");
        }
        return new Intl.DateTimeFormat(locale, {
          dateStyle: "medium",
          timeStyle: "short",
        }).format(value * 1000);
      },
      formatNumber: (value) => new Intl.NumberFormat(locale).format(value),
    };
  }, [locale]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  return useContext(I18nContext);
}

export type { Locale, MessageCatalog, TranslationValues, Translator } from "@/i18n/types";
export {
  DEFAULT_LOCALE,
  LOCALE_STORAGE_KEY,
  resolveInitialLocale,
  resolveLocale,
  supportedLocales,
};
