export const supportedLocales = ["en-US", "zh-CN"] as const;

export type Locale = (typeof supportedLocales)[number];
export type MessageCatalog = Record<string, string>;
export type TranslationValues = Record<string, string | number | null | undefined>;
export type Translator = (message: string, values?: TranslationValues) => string;
