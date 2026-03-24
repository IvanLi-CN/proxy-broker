import type { MessageCatalog } from "@/i18n/types";

export const enUS: MessageCatalog = {
  "Overview hero title":
    "Run the operator plane like a control room instead of a stack of scattered notes.",
  "Overview hero description":
    "This surface keeps subscription loading, health signals, and next-step guidance in one place so you can move from feed refresh to listener orchestration without rechecking the basics.",
  "Tasks hero title":
    "Treat the subscription maintenance loop like a live board instead of an invisible cron.",
  "Tasks hero description":
    "This route turns the automation layer into an operator-facing surface: sync cadence, metadata refresh pressure, failure context, and per-run event streams all stay visible here.",
  "IP Extract hero title": "Cut the pool down to a shortlist you can actually trust.",
  "IP Extract hero description":
    "Use the filter builder to move from broad geography hints to a tighter candidate deck with clear probe, latency, and recency signals. The goal is not more rows, but better rows.",
  "Sessions hero title":
    "Turn the shortlist into live listeners without losing the wider operational view.",
  "Sessions hero description":
    "Open one deterministic listener or stage a transactional batch while keeping the live deck in sight so teardown decisions stay fast and low risk.",
  "validation.source_value_required": "Source value is required.",
  "error.api.with_code": "{code}: {message}",
  "error.api.subscription_invalid": "Subscription payload is invalid.",
  "error.api.subscription_fetch_failed": "Subscription source is temporarily unreachable.",
  "error.api.ip_not_found": "No matching candidate IP was found.",
  "error.api.ip_conflict_blacklist":
    "These IPs appear in both the include list and the blacklist: {conflicts}.",
  "error.api.session_not_found": "The requested session could not be found.",
  "error.api.port_in_use": "That port is already in use.",
  "error.api.profile_exists": "Profile already exists",
  "error.api.profile_not_found": "The requested profile could not be found.",
  "error.api.invalid_port": "The requested port is invalid.",
  "error.api.invalid_request": "The request payload is invalid.",
  "error.api.authentication_required": "Authentication is required.",
  "error.api.admin_required": "Admin access is required.",
  "error.api.api_key_invalid": "The API key is invalid.",
  "error.api.api_key_revoked": "The API key has been revoked.",
  "error.api.api_key_not_found": "The API key could not be found.",
  "error.api.task_run_not_found": "The task run could not be found.",
  "error.api.profile_access_denied": "The current identity cannot access this profile.",
  "error.api.mihomo_unavailable": "The mihomo runtime is currently unavailable.",
  "error.api.batch_open_failed": "Batch open failed.",
  "error.api.internal_error": "An internal error occurred.",
  "error.api.serialization_error": "Response serialization failed.",
  "error.api.http_error": "The request failed (HTTP {status}).",
  "error.api.with_reason": "{message} Reason: {reason}",
  "error.task.fallback": "Task run failed.",
  "error.task.summary_reason_prefix": "Summary reason: {reason}",
};
