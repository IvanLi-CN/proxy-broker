import { describe, expect, it } from "vitest";

import { zhCN } from "@/i18n/messages/zh-CN";
import {
  formatTaskEventMessage,
  formatTaskPayloadKey,
  localizeTaskPayload,
} from "@/lib/tasks-view";

const t = (message: string, values?: Record<string, string | number | null | undefined>) =>
  (zhCN[message] ?? message).replace(/\{(\w+)\}/g, (_, key: string) => {
    const value = values?.[key];
    return value == null ? `{${key}}` : String(value);
  });

describe("formatTaskEventMessage", () => {
  it("localizes known backend event messages", () => {
    expect(formatTaskEventMessage("Refreshing probe metadata.", t)).toBe("正在刷新探测元数据。");
    expect(formatTaskEventMessage("Subscription sync finished with 12 new IP(s).", t)).toBe(
      "订阅同步完成，新增 12 个 IP。",
    );
  });
});

describe("formatTaskPayloadKey", () => {
  it("maps known payload keys to translated labels", () => {
    expect(formatTaskPayloadKey("targeted_ips", t)).toBe("目标 IP 数");
    expect(formatTaskPayloadKey("reason", t)).toBe("原因");
  });
});

describe("localizeTaskPayload", () => {
  it("recursively localizes payload keys", () => {
    expect(
      localizeTaskPayload(
        {
          targeted_ips: 12,
          nested: {
            skipped_cached: 3,
          },
        },
        t,
      ),
    ).toEqual({
      "目标 IP 数": 12,
      nested: {
        已跳过缓存项: 3,
      },
    });
  });
});
