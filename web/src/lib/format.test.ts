import { describe, expect, it } from "vitest";

import { buildExtractRequest, buildOpenSessionRequest, splitListInput } from "@/lib/format";

describe("splitListInput", () => {
  it("accepts commas and newlines", () => {
    expect(splitListInput("JP, US\nSG")).toEqual(["JP", "US", "SG"]);
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

describe("buildOpenSessionRequest", () => {
  it("builds a direct IP request with selector details", () => {
    expect(
      buildOpenSessionRequest({
        specifiedIp: "203.0.113.10",
        desiredPort: "10080",
        countryCodes: "JP",
        cities: "Tokyo",
        selectorSpecifiedIps: "203.0.113.10",
        blacklistIps: "",
        limit: "1",
        sortMode: "mru",
      }),
    ).toEqual({
      specified_ip: "203.0.113.10",
      desired_port: 10080,
      selector: {
        country_codes: ["JP"],
        cities: ["Tokyo"],
        specified_ips: ["203.0.113.10"],
        limit: 1,
        sort_mode: "mru",
      },
    });
  });
});
