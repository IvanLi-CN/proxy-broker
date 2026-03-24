import { describe, expect, it } from "vitest";

import { zhCN } from "@/i18n/messages/zh-CN";
import { ApiError } from "@/lib/api";
import { formatApiErrorMessage, formatTaskErrorMessage } from "@/lib/error-messages";

const t = (message: string, values?: Record<string, string | number | null | undefined>) =>
  (zhCN[message] ?? message).replace(/\{(\w+)\}/g, (_, key: string) => {
    const value = values?.[key];
    return value == null ? `{${key}}` : String(value);
  });

describe("formatApiErrorMessage", () => {
  it("maps known backend codes to translated messages", () => {
    const error = new ApiError(400, {
      code: "subscription_invalid",
      message: "subscription payload invalid",
    });

    expect(formatApiErrorMessage(error, t)).toBe("subscription_invalid: 订阅载荷无效。");
  });

  it("keeps the backend message when the code is unknown", () => {
    const error = new ApiError(500, {
      code: "custom_backend_error",
      message: "upstream exploded",
    });

    expect(formatApiErrorMessage(error, t)).toBe("custom_backend_error: upstream exploded");
  });
});

describe("formatTaskErrorMessage", () => {
  it("prefers known task error codes when available", () => {
    expect(
      formatTaskErrorMessage(
        {
          error_code: "mihomo_unavailable",
          error_message: "mihomo runtime unavailable: controller offline",
          summary_json: null,
        },
        t,
      ),
    ).toContain("mihomo");
  });

  it("falls back to summary reasons for uncoded task failures", () => {
    expect(
      formatTaskErrorMessage(
        {
          error_code: null,
          error_message: null,
          summary_json: { reason: "no candidate edge survived probing" },
        },
        t,
      ),
    ).toBe("摘要原因：no candidate edge survived probing");
  });
});
