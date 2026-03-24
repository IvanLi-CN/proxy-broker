import type { Locale, Translator } from "@/i18n";
import type { ExtractIpRequest, OpenSessionRequest, SortMode } from "@/lib/types";

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
