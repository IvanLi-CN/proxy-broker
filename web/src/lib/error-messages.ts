import type { Translator } from "@/i18n";
import { ApiError } from "@/lib/api";
import type { TaskRunSummary } from "@/lib/types";

function extractReason(message: string, prefix: string) {
  return message.startsWith(prefix) ? message.slice(prefix.length).trim() : message;
}

function withOptionalReason(t: Translator, baseMessage: string, reason?: string | null) {
  const trimmedReason = reason?.trim();
  if (!trimmedReason || trimmedReason === baseMessage) {
    return baseMessage;
  }
  return t("error.api.with_reason", {
    message: baseMessage,
    reason: trimmedReason,
  });
}

function formatKnownApiError(t: Translator, error: ApiError) {
  switch (error.code) {
    case "subscription_invalid":
      return t("error.api.subscription_invalid");
    case "subscription_fetch_failed":
      return withOptionalReason(
        t,
        t("error.api.subscription_fetch_failed"),
        extractReason(error.message, "subscription source not reachable:"),
      );
    case "ip_not_found":
      return t("error.api.ip_not_found");
    case "ip_conflict_blacklist": {
      const conflicts = Array.isArray((error.details as { conflicts?: unknown } | null)?.conflicts)
        ? (error.details as { conflicts: unknown[] }).conflicts
            .map((item) => String(item))
            .join(", ")
        : null;
      return conflicts
        ? t("error.api.ip_conflict_blacklist", { conflicts })
        : t("error.api.ip_conflict_blacklist", { conflicts: error.message });
    }
    case "session_not_found":
      return t("error.api.session_not_found");
    case "port_in_use":
      return t("error.api.port_in_use");
    case "profile_exists":
      return t("error.api.profile_exists");
    case "profile_not_found":
      return t("error.api.profile_not_found");
    case "invalid_port":
      return t("error.api.invalid_port");
    case "invalid_request":
      return withOptionalReason(
        t,
        t("error.api.invalid_request"),
        extractReason(error.message, "invalid request:"),
      );
    case "authentication_required":
      return t("error.api.authentication_required");
    case "admin_required":
      return t("error.api.admin_required");
    case "api_key_invalid":
      return t("error.api.api_key_invalid");
    case "api_key_revoked":
      return t("error.api.api_key_revoked");
    case "api_key_not_found":
      return t("error.api.api_key_not_found");
    case "task_run_not_found":
      return t("error.api.task_run_not_found");
    case "profile_access_denied":
      return t("error.api.profile_access_denied");
    case "mihomo_unavailable":
      return withOptionalReason(
        t,
        t("error.api.mihomo_unavailable"),
        extractReason(error.message, "mihomo runtime unavailable:"),
      );
    case "batch_open_failed":
      return t("error.api.batch_open_failed");
    case "internal_error":
      return withOptionalReason(
        t,
        t("error.api.internal_error"),
        extractReason(error.message, "internal error:"),
      );
    case "serialization_error":
      return withOptionalReason(t, t("error.api.serialization_error"), error.message);
    default:
      if (error.code.startsWith("http_")) {
        const status = error.code.slice(5);
        return withOptionalReason(t, t("error.api.http_error", { status }), error.message);
      }
      return error.message;
  }
}

export function formatApiErrorMessage(error: unknown, t: Translator) {
  if (!(error instanceof ApiError)) {
    return t("Unexpected request error");
  }

  return t("error.api.with_code", {
    code: error.code,
    message: formatKnownApiError(t, error),
  });
}

export function formatTaskErrorMessage(
  run: Pick<TaskRunSummary, "error_code" | "error_message" | "summary_json">,
  t: Translator,
) {
  if (run.error_code) {
    const apiError = new ApiError(500, {
      code: run.error_code,
      message: run.error_message ?? run.error_code,
    });
    return formatKnownApiError(t, apiError);
  }

  if (typeof run.summary_json?.reason === "string") {
    return t("error.task.summary_reason_prefix", {
      reason: String(run.summary_json.reason),
    });
  }

  return run.error_message ?? t("error.task.fallback");
}
