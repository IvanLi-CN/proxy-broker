import type { Locale, Translator } from "@/i18n";
import type {
  ExtractIpRequest,
  OpenSessionRequest,
  SessionSelectionMode,
  SortMode,
  SubscriptionMetadata,
} from "@/lib/types";

const zhGeoLabels: Record<string, string> = {
  Japan: "日本",
  "United States": "美国",
  Tokyo: "东京",
  Osaka: "大阪",
  Chiyoda: "千代田",
  California: "加利福尼亚",
  "San Jose": "圣何塞",
};

const COUNTRY_CODE_PATTERN = /^[A-Za-z]{2}$/;
const SPECIAL_COUNTRY_CODES = new Set(["A1", "A2", "O1"]);

export function splitListInput(value: string) {
  return value
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

export function optionalNumber(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return undefined;
  }
  const parsed = Number.parseInt(trimmed, 10);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function uniqueItems(items: string[]) {
  return Array.from(new Set(items.map((item) => item.trim()).filter(Boolean)));
}

function normalizeOverlapKey(value: string) {
  return value.trim().toLowerCase();
}

export function findOverlappingValues(left: string[], right: string[]) {
  const rightKeys = new Set(right.map((value) => normalizeOverlapKey(value)).filter(Boolean));
  return uniqueItems(left).filter((value) => rightKeys.has(normalizeOverlapKey(value)));
}

function parseCitySelectionToken(value: string) {
  const trimmed = value.trim();
  const separatorIndex = trimmed.indexOf("::");
  if (separatorIndex < 0) {
    return null;
  }
  const countryCode = trimmed.slice(0, separatorIndex).trim().toUpperCase();
  const city = trimmed.slice(separatorIndex + 2).trim();
  if (!countryCode || !city) {
    return null;
  }
  return { countryCode, city };
}

export function filterCitySelectionsByCountry(cities: string[], countryCodes: string[]) {
  const allowed = new Set(countryCodes.map((code) => code.trim().toUpperCase()).filter(Boolean));
  if (allowed.size === 0) {
    return uniqueItems(cities);
  }
  return uniqueItems(
    cities.filter((value) => {
      const parsed = parseCitySelectionToken(value);
      return parsed ? allowed.has(parsed.countryCode) : true;
    }),
  );
}

export function formatTimestamp(locale: Locale, t: Translator, epoch?: number | null) {
  if (!epoch) {
    return t("Never");
  }
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(epoch * 1000);
}

export function formatLatency(locale: Locale, t: Translator, value?: number | null) {
  if (value == null) {
    return t("N/A");
  }
  return `${new Intl.NumberFormat(locale).format(value)} ms`;
}

export function formatDataSize(locale: Locale, t: Translator, value?: number | null) {
  if (value == null) {
    return null;
  }
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  let size = value;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  const formatted =
    size >= 100 || unitIndex === 0
      ? new Intl.NumberFormat(locale, { maximumFractionDigits: 0 }).format(size)
      : new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(size);
  return t("{value} {unit}", { value: formatted, unit: units[unitIndex] ?? "B" });
}

export function formatSubscriptionQuota(
  locale: Locale,
  t: Translator,
  metadata?: SubscriptionMetadata | null,
) {
  if (!metadata) {
    return null;
  }
  const remaining = formatDataSize(locale, t, metadata.remaining_bytes);
  const total = formatDataSize(locale, t, metadata.total_bytes);
  if (remaining && total) {
    return t("Remaining {remaining} / {total}", { remaining, total });
  }
  const used = formatDataSize(locale, t, metadata.used_bytes);
  if (used && total) {
    return t("Used {used} / {total}", { used, total });
  }
  return total ? t("Total {total}", { total }) : null;
}

export function formatSubscriptionExpire(
  locale: Locale,
  t: Translator,
  metadata?: SubscriptionMetadata | null,
) {
  if (!metadata?.expire_at) {
    return null;
  }
  return t("Expires {time}", { time: formatTimestamp(locale, t, metadata.expire_at) });
}

export function buildSubscriptionMetadataBullets(
  locale: Locale,
  t: Translator,
  response: {
    resolved_name?: string | null;
    resolved_name_source?: string | null;
    subscription_metadata?: SubscriptionMetadata | null;
    warnings?: string[];
  },
) {
  const bullets: string[] = [];
  const resolvedName = response.resolved_name?.trim();
  if (resolvedName) {
    bullets.push(t("Resolved name: {name}", { name: resolvedName }));
  }
  const sourceTitle = response.subscription_metadata?.source_title?.trim();
  if (sourceTitle && sourceTitle !== resolvedName) {
    bullets.push(t("Source title: {title}", { title: sourceTitle }));
  }
  const quota = formatSubscriptionQuota(locale, t, response.subscription_metadata);
  if (quota) {
    bullets.push(quota);
  }
  const expire = formatSubscriptionExpire(locale, t, response.subscription_metadata);
  if (expire) {
    bullets.push(expire);
  }
  for (const warning of response.warnings ?? []) {
    bullets.push(formatOperatorWarning(t, warning));
  }
  return bullets;
}

export function formatListSummary(t: Translator, items: string[]) {
  if (items.length === 0) {
    return t("Not set");
  }
  return items.join(" / ");
}

export function formatSortMode(sortMode: SortMode, t: Translator) {
  switch (sortMode) {
    case "lru":
      return t("Least recently used first");
    case "mru":
      return t("Most recently used first");
  }
}

export function formatGeoLabel(locale: Locale, value?: string | null) {
  if (!value) {
    return null;
  }
  if (locale !== "zh-CN") {
    return value;
  }
  return zhGeoLabels[value] ?? value;
}

export function normalizeCountryCode(countryCode?: string | null) {
  if (!countryCode) {
    return null;
  }
  const normalized = countryCode.trim().toUpperCase();
  return COUNTRY_CODE_PATTERN.test(normalized) || SPECIAL_COUNTRY_CODES.has(normalized)
    ? normalized
    : null;
}

export function formatCountryName(
  locale: Locale,
  countryCode?: string | null,
  fallbackName?: string | null,
) {
  const normalizedCountryCode = normalizeCountryCode(countryCode);
  if (normalizedCountryCode) {
    try {
      const name = new Intl.DisplayNames([locale], { type: "region" }).of(normalizedCountryCode);
      if (name) {
        return name;
      }
    } catch {
      // Ignore invalid region subtags and fall back to the provided display name.
    }
  }

  return formatGeoLabel(locale, fallbackName) ?? fallbackName ?? normalizedCountryCode ?? null;
}

export function formatOperatorWarning(t: Translator, warning: string) {
  const dnsFallbackMatch = warning.match(
    /^proxy `([^`]+)` dns resolve failed, reused (\d+) cached ip\(s\)$/i,
  );
  if (dnsFallbackMatch) {
    const [, proxyName, count] = dnsFallbackMatch;
    return t("Proxy {proxyName} DNS resolution failed; reused {count} cached IPs.", {
      proxyName,
      count,
    });
  }

  const reusedMatch = warning.match(/^(.+?) reused (\d+) cached IPs?$/i);
  if (reusedMatch) {
    const [, proxyName, count] = reusedMatch;
    return t("Proxy {proxyName} reused {count} cached IPs.", { proxyName, count });
  }

  const filteredInfoMatch = warning.match(/^filtered informational subscription entry `([^`]+)`$/i);
  if (filteredInfoMatch) {
    const [, name] = filteredInfoMatch;
    return t("Filtered informational subscription entry {name}.", { name });
  }

  return warning;
}

export function formatHealthStatus(status: string, t: Translator) {
  return status.trim().toLowerCase() === "ok" ? t("Healthy") : status.toUpperCase();
}

export function buildExtractRequest(values: {
  countryCodes: string;
  cities: string;
  specifiedIps: string;
  blacklistIps: string;
  limit: string;
  sortMode: SortMode;
}): ExtractIpRequest {
  const request: ExtractIpRequest = {
    sort_mode: values.sortMode,
  };
  const mappings = [
    ["country_codes", splitListInput(values.countryCodes)],
    ["cities", splitListInput(values.cities)],
    ["specified_ips", splitListInput(values.specifiedIps)],
    ["blacklist_ips", splitListInput(values.blacklistIps)],
  ] as const;

  for (const [key, list] of mappings) {
    if (list.length > 0) {
      request[key] = list;
    }
  }

  const limit = optionalNumber(values.limit);
  if (limit !== undefined) {
    request.limit = limit;
  }

  return request;
}

export function buildOpenSessionRequest(values: {
  selectionMode: SessionSelectionMode;
  desiredPort: string;
  countryCodes: string[];
  cities: string[];
  specifiedIps: string[];
  excludedIps: string[];
  sortMode: SortMode;
}): OpenSessionRequest {
  const request: OpenSessionRequest = {
    selection_mode: values.selectionMode,
    sort_mode: values.sortMode,
  };

  if (values.selectionMode === "geo") {
    const parsedCitySelections = values.cities.map((value) => ({
      value,
      parsed: parseCitySelectionToken(value),
    }));
    const tokenizedCityNames = new Set(
      parsedCitySelections
        .map((entry) => entry.parsed?.city.trim().toLocaleLowerCase())
        .filter((value): value is string => Boolean(value)),
    );
    const tokenizedCities = parsedCitySelections
      .map((entry) => (entry.parsed ? `${entry.parsed.countryCode}::${entry.parsed.city}` : null))
      .filter((value): value is string => Boolean(value));
    const plainCities = parsedCitySelections
      .map((entry) => (entry.parsed ? null : entry.value.trim()))
      .filter((value): value is string => Boolean(value))
      .filter((value) => !tokenizedCityNames.has(value.toLocaleLowerCase()));

    const countryCodes = uniqueItems(values.countryCodes);
    const cities = uniqueItems([...tokenizedCities, ...plainCities]);

    if (countryCodes.length > 0) {
      request.country_codes = countryCodes;
    }
    if (cities.length > 0) {
      request.cities = cities;
    }
  }

  if (values.selectionMode === "ip") {
    const specifiedIps = uniqueItems(values.specifiedIps);
    if (specifiedIps.length > 0) {
      request.specified_ips = specifiedIps;
    }
  }

  const excludedIps = uniqueItems(values.excludedIps);
  if (excludedIps.length > 0) {
    request.excluded_ips = excludedIps;
  }

  const desiredPort = optionalNumber(values.desiredPort);
  if (desiredPort !== undefined) {
    request.desired_port = desiredPort;
  }
  return request;
}
