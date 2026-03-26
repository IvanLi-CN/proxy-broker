import { describe, expect, it } from "vitest";

import { zhCN } from "@/i18n/messages/zh-CN";
import {
  buildExtractRequest,
  buildOpenSessionRequest,
  filterCitySelectionsByCountry,
  findOverlappingValues,
  formatLatency,
  formatTimestamp,
  splitListInput,
} from "@/lib/format";

const t = (message: string) => zhCN[message] ?? message;

describe("splitListInput", () => {
  it("accepts commas and newlines", () => {
    expect(splitListInput("JP, US\nSG")).toEqual(["JP", "US", "SG"]);
  });
});

describe("filterCitySelectionsByCountry", () => {
  it("drops stale encoded city tokens while preserving plain city selections", () => {
    expect(
      filterCitySelectionsByCountry(["JP::Tokyo", "US::San Jose", "Tokyo", "Osaka"], ["US"]),
    ).toEqual(["US::San Jose", "Tokyo", "Osaka"]);
  });
});

describe("buildExtractRequest", () => {
  it("drops empty lists and parses numeric limit", () => {
    expect(
      buildExtractRequest({
        countryCodes: "JP, US",
        cities: "Tokyo",
        specifiedIps: "",
        blacklistIps: "198.51.100.42",
        limit: "20",
        sortMode: "lru",
      }),
    ).toEqual({
      country_codes: ["JP", "US"],
      cities: ["Tokyo"],
      blacklist_ips: ["198.51.100.42"],
      limit: 20,
      sort_mode: "lru",
    });
  });
});

describe("findOverlappingValues", () => {
  it("matches overlapping values case-insensitively after trimming", () => {
    expect(
      findOverlappingValues([" 203.0.113.10 ", "203.0.113.11"], ["203.0.113.10", " 203.0.113.12 "]),
    ).toEqual(["203.0.113.10"]);
  });
});

describe("buildOpenSessionRequest", () => {
  it("builds a flattened ip-targeted session request", () => {
    expect(
      buildOpenSessionRequest({
        selectionMode: "ip",
        desiredPort: "10080",
        countryCodes: ["JP"],
        cities: ["Tokyo"],
        specifiedIps: ["203.0.113.10", "203.0.113.11"],
        excludedIps: ["198.51.100.42"],
        sortMode: "mru",
      }),
    ).toEqual({
      selection_mode: "ip",
      desired_port: 10080,
      specified_ips: ["203.0.113.10", "203.0.113.11"],
      excluded_ips: ["198.51.100.42"],
      sort_mode: "mru",
    });
  });

  it("keeps encoded city selections opaque", () => {
    expect(
      buildOpenSessionRequest({
        selectionMode: "geo",
        desiredPort: "",
        countryCodes: [],
        cities: ["JP::Tokyo", "FR::Paris", "Paris"],
        specifiedIps: [],
        excludedIps: [],
        sortMode: "lru",
      }),
    ).toEqual({
      selection_mode: "geo",
      cities: ["JP::Tokyo", "FR::Paris"],
      sort_mode: "lru",
    });
  });

  it("preserves plain cities when mixed with encoded selections for other cities", () => {
    expect(
      buildOpenSessionRequest({
        selectionMode: "geo",
        desiredPort: "",
        countryCodes: ["JP"],
        cities: ["Osaka", "JP::Tokyo"],
        specifiedIps: [],
        excludedIps: [],
        sortMode: "lru",
      }),
    ).toEqual({
      selection_mode: "geo",
      country_codes: ["JP"],
      cities: ["JP::Tokyo", "Osaka"],
      sort_mode: "lru",
    });
  });

  it("drops plain duplicates once an encoded city for the same label exists", () => {
    expect(
      buildOpenSessionRequest({
        selectionMode: "geo",
        desiredPort: "",
        countryCodes: ["JP"],
        cities: ["Osaka", "JP::Osaka"],
        specifiedIps: [],
        excludedIps: [],
        sortMode: "lru",
      }),
    ).toEqual({
      selection_mode: "geo",
      country_codes: ["JP"],
      cities: ["JP::Osaka"],
      sort_mode: "lru",
    });
  });
});

describe("formatTimestamp", () => {
  it("uses the requested locale for timestamps", () => {
    const epoch = 1_735_689_600;

    expect(formatTimestamp("en-US", t, epoch)).toBe(
      new Intl.DateTimeFormat("en-US", {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(epoch * 1000),
    );
    expect(formatTimestamp("zh-CN", t, epoch)).toBe(
      new Intl.DateTimeFormat("zh-CN", {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(epoch * 1000),
    );
  });

  it("falls back to a translated Never label for empty values", () => {
    expect(formatTimestamp("zh-CN", t, null)).toBe("从未");
  });
});

describe("formatLatency", () => {
  it("formats latency with locale-aware numbers", () => {
    expect(formatLatency("zh-CN", t, 12345)).toBe(
      `${new Intl.NumberFormat("zh-CN").format(12345)} ms`,
    );
  });
});
