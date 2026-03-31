import { describe, expect, it } from "vitest";

import {
  buildNodeExportRequest,
  buildNodeListQuery,
  buildNodeOpenSessionsRequest,
  defaultNodeFilterState,
  groupNodeItems,
} from "@/lib/nodes-view";
import { nodesFixture } from "@/mocks/fixtures";

describe("nodes-view helpers", () => {
  it("builds a server query from filter state", () => {
    const query = buildNodeListQuery({
      ...defaultNodeFilterState,
      query: "tokyo",
      countryCodes: "JP, US",
      regions: "Tokyo",
      cities: "Chiyoda",
      probeStatus: "reachable",
      sortBy: "latency",
      sortOrder: "asc",
      page: 2,
      pageSize: 50,
    });

    expect(query).toEqual({
      query: "tokyo",
      country_codes: ["JP", "US"],
      regions: ["Tokyo"],
      cities: ["Chiyoda"],
      probe_status: "reachable",
      session_presence: "any",
      ip_family: "any",
      sort_by: "latency",
      sort_order: "asc",
      page: 2,
      page_size: 50,
    });
  });

  it("groups the current page by subscription label", () => {
    const groups = groupNodeItems(nodesFixture.items, "group_by_subscription");

    expect(groups).toHaveLength(1);
    expect(groups[0]?.items).toHaveLength(3);
    expect(groups[0]?.label).toContain("https://example.com/subscription.yaml");
  });

  it("builds selected and all-filtered action payloads", () => {
    const selectedExport = buildNodeExportRequest(
      "selected",
      ["node-a"],
      {
        query: "tokyo",
        page: 3,
        page_size: 25,
      },
      "link_lines",
    );
    const filteredOpen = buildNodeOpenSessionsRequest("all_filtered", [], {
      query: "tokyo",
      page: 3,
      page_size: 25,
    });

    expect(selectedExport).toEqual({
      node_ids: ["node-a"],
      format: "link_lines",
    });
    expect(filteredOpen).toEqual({
      all_filtered: true,
      query: {
        query: "tokyo",
        page: undefined,
        page_size: undefined,
      },
      ip_family_priority: "ipv4_first",
    });
  });
});
