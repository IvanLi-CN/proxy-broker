import type { Locale, Translator } from "@/i18n";
import type { ExtractIpRequest, OpenSessionRequest, SortMode } from "@/lib/types";

const zhGeoLabels: Record<string, string> = {
  Japan: "日本",
  "United States": "美国",
  Tokyo: "东京",
  Osaka: "大阪",
  Chiyoda: "千代田",
  California: "加利福尼亚",
  "San Jose": "圣何塞",
};

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

export function formatCountryName(
  locale: Locale,
  countryCode?: string | null,
  fallbackName?: string | null,
) {
  if (countryCode) {
    const name = new Intl.DisplayNames([locale], { type: "region" }).of(countryCode);
    if (name) {
      return name;
    }
  }

  return formatGeoLabel(locale, fallbackName) ?? fallbackName ?? null;
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
  specifiedIp: string;
  desiredPort: string;
  countryCodes: string;
  cities: string;
  selectorSpecifiedIps: string;
  blacklistIps: string;
  limit: string;
  sortMode: SortMode;
}): OpenSessionRequest {
  const selector = buildExtractRequest({
    countryCodes: values.countryCodes,
    cities: values.cities,
    specifiedIps: values.selectorSpecifiedIps,
    blacklistIps: values.blacklistIps,
    limit: values.limit,
    sortMode: values.sortMode,
  });

  const request: OpenSessionRequest = {};
  const specifiedIp = values.specifiedIp.trim();
  if (specifiedIp) {
    request.specified_ip = specifiedIp;
  }
  if (Object.keys(selector).length > 0) {
    request.selector = selector;
  }
  const desiredPort = optionalNumber(values.desiredPort);
  if (desiredPort !== undefined) {
    request.desired_port = desiredPort;
  }
  return request;
}
