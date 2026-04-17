import type { MessageCatalog } from "@/i18n/types";

export const enUS: MessageCatalog = {
  Healthy: "Healthy",
  "Least recently used first": "Least recently used first",
  "Most recently used first": "Most recently used first",
  "Default sort: {sortMode}": "Default sort: {sortMode}",
  "Enter one city per line": "Enter one city per line",
  "Proxy {proxyName} DNS resolution failed; reused {count} cached IPs.":
    "Proxy {proxyName} DNS resolution failed; reused {count} cached IPs.",
  "Proxy {proxyName} reused {count} cached IPs.": "Proxy {proxyName} reused {count} cached IPs.",
  "Targeted IPs": "Targeted IPs",
  "Probed IPs": "Probed IPs",
  "Geo records updated": "Geo records updated",
  "Cached entries skipped": "Cached entries skipped",
  "Loaded proxies": "Loaded proxies",
  "Distinct IPs": "Distinct IPs",
  Reason: "Reason",
  "Refreshing subscription feed for profile.": "Refreshing subscription feed for profile.",
  "Refreshing probe metadata.": "Refreshing probe metadata.",
  "Task run queued.": "Task run queued.",
  "Task run completed successfully.": "Task run completed successfully.",
  "Task run skipped.": "Task run skipped.",
  "Task run failed.": "Task run failed.",
  "Task run is running.": "Task run is running.",
  "Subscription sync finished with {count} new IPs.":
    "Subscription sync finished with {count} new IPs.",
  "Pick one simple targeting mode, keep the port optional, and let the backend open the listener from the first surviving candidate.":
    "Pick one simple targeting mode, keep the port optional, and let the backend open the listener from the first surviving candidate.",
  "optional port": "optional port",
  "Stage multiple open-session requests with the same simplified targeting model, then let the backend roll the whole set back if any row fails.":
    "Stage multiple open-session requests with the same simplified targeting model, then let the backend roll the whole set back if any row fails.",
  "One row, one listener; all rows still succeed or fail together.":
    "One row, one listener; all rows still succeed or fail together.",
  Advanced: "Advanced",
  optional: "optional",
  "validation.source_value_required": "Source value is required.",
  "validation.content_required": "Nodes content is required.",
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
  "The control surface only exposes Overview, Tasks, Proxies, IP Extract, and Sessions right now.":
    "The control surface only exposes Overview, Tasks, Proxies, IP Extract, and Sessions right now.",
  Global: "Global",
  "Current config": "Current config",
  "Config ID": "Config ID",
  "Search configs or type a new ID": "Search configs or type a new ID",
  "Loading configs...": "Loading configs...",
  Contexts: "Contexts",
  "Known configs": "Known configs",
  "Shared pool and allocation control across every profile.":
    "Shared pool and allocation control across every profile.",
  "Start an empty config catalog entry and switch to it immediately.":
    "Start an empty config catalog entry and switch to it immediately.",
  "No matching configs. Type a new ID to create one.":
    "No matching configs. Type a new ID to create one.",
  "Search the catalog or create a new empty config before loading any feed.":
    "Search the catalog or create a new empty config before loading any feed.",
  "Profile workspace": "Profile workspace",
  "error.api.proxy_inventory_node_not_found": "The imported proxy node could not be found.",
  Proxies: "Proxies",
  "Global proxies": "Global proxies",
  "Shared pool and cross-profile allocations": "Shared pool and cross-profile allocations",
  "Manage local imports, global pool usage, and allocations":
    "Manage local imports, global pool usage, and allocations",
  "Profile only": "Profile only",
  "Select a concrete profile to use this workspace.":
    "Select a concrete profile to use this workspace.",
  "Manage the global pool, profile imports, and allocations":
    "Manage the global pool, profile imports, and allocations",
  "Manage the global pool and cross-profile allocations":
    "Manage the global pool and cross-profile allocations",
  "The proxies workspace is restricted to the admin operator plane because it can change global pool allocation.":
    "The proxies workspace is restricted to the admin operator plane because it can change global pool allocation.",
  "Manage the global pool, the current profile's local imports, and where each imported node is allocated.":
    "Manage the global pool, the current profile's local imports, and where each imported node is allocated.",
  "Manage the shared global pool and cross-profile allocations from one place.":
    "Manage the shared global pool and cross-profile allocations from one place.",
  "Manage the shared global pool and cross-profile allocations from one place. Profile-local imports and usage stay inside each profile overview.":
    "Manage the shared global pool and cross-profile allocations from one place. Profile-local imports and usage stay inside each profile overview.",
  "Keep the global pool in its own workspace, then manage local imports and policy separately for the current profile.":
    "Keep the global pool in its own workspace, then manage local imports and policy separately for the current profile.",
  "Global workspace": "Global workspace",
  "Global entry": "Global entry",
  "Current profile workspace": "Current profile workspace",
  "Shared proxy administration": "Shared proxy administration",
  "Global pool and cross-profile allocations live here.":
    "Global pool and cross-profile allocations live here.",
  "Enter from the left nav. This page does not follow the current profile.":
    "Enter from the left nav. This page does not follow the current profile.",
  "Global operator plane": "Global operator plane",
  "Shared global pool": "Shared global pool",
  "Global scope": "Global scope",
  "Applies to every profile that keeps global pool enabled.":
    "Applies to every profile that keeps global pool enabled.",
  "Applies across all profiles.": "Applies across all profiles.",
  "allocation defaults to global": "allocation defaults to global",
  "allocation defaults to {profileId}": "allocation defaults to {profileId}",
  "remote fetch": "remote fetch",
  "host file": "host file",
  "Re-import restores nodes that still exist upstream.":
    "Re-import restores nodes that still exist upstream.",
  "Imported {count} global proxies": "Imported {count} global proxies",
  "Imported {count} profile proxies for {profileId}":
    "Imported {count} profile proxies for {profileId}",
  "Import global proxy pool": "Import global proxy pool",
  "Import type": "Import type",
  Subscription: "Subscription",
  Nodes: "Nodes",
  Name: "Name",
  "subscription source": "subscription source",
  "node group": "node group",
  "Leave blank to use the source domain when possible":
    "Leave blank to use the source domain when possible",
  "Leave blank to group nodes by the first proxy name":
    "Leave blank to group nodes by the first proxy name",
  "Optional. Leave blank to auto-name from the ASCII domain on URL imports; otherwise the list falls back to the import ID.":
    "Optional. Leave blank to auto-name from the ASCII domain on URL imports; otherwise the list falls back to the import ID.",
  "Optional. Leave blank to auto-name the node group from its first proxy; if that is unavailable, the list falls back to the import ID.":
    "Optional. Leave blank to auto-name the node group from its first proxy; if that is unavailable, the list falls back to the import ID.",
  "Nodes content": "Nodes content",
  "Paste one or more Clash-compatible nodes as `proxies:` YAML or a plain list. Everything in the textarea is imported as one original node group.":
    "Paste one or more Clash-compatible nodes as `proxies:` YAML or a plain list. Everything in the textarea is imported as one original node group.",
  "Each submit creates one original import group that can later be reallocated or deleted as a whole.":
    "Each submit creates one original import group that can later be reallocated or deleted as a whole.",
  "Batch node imports keep every pasted node inside the same allocation group.":
    "Batch node imports keep every pasted node inside the same allocation group.",
  "Import one source into the shared global pool. Profiles that keep global usage enabled will inherit these nodes immediately.":
    "Import one source into the shared global pool. Profiles that keep global usage enabled will inherit these nodes immediately.",
  "Import one upstream into the shared pool. Profiles that keep global usage enabled inherit these nodes immediately.":
    "Import one upstream into the shared pool. Profiles that keep global usage enabled inherit these nodes immediately.",
  "Import one subscription source or one node group into the shared pool. Profiles that keep global usage enabled inherit these nodes immediately.":
    "Import one subscription source or one node group into the shared pool. Profiles that keep global usage enabled inherit these nodes immediately.",
  "Import global pool": "Import global pool",
  "Global pool updated": "Global pool updated",
  "Imported {proxyCount} proxies across {ipCount} distinct IPs into the global pool.":
    "Imported {proxyCount} proxies across {ipCount} distinct IPs into the global pool.",
  "Manage the shared global pool and every profile allocation from here.":
    "Manage the shared global pool and every profile allocation from here.",
  "Global pool and configuration allocations": "Global pool and configuration allocations",
  "Original imports": "Original imports",
  "Allocate by original import source. Subscription rows are reassigned or deleted as a whole; profile composition still happens from their member nodes behind the scenes.":
    "Allocate by original import source. Subscription rows are reassigned or deleted as a whole; profile composition still happens from their member nodes behind the scenes.",
  "{count} import": "{count} import",
  "{count} imports": "{count} imports",
  "loading imports": "loading imports",
  "imports live": "imports live",
  "Allocation and deletion now happen at the original import level. Re-importing the same source only refreshes that import and leaves other imports untouched.":
    "Allocation and deletion now happen at the original import level. Re-importing the same source only refreshes that import and leaves other imports untouched.",
  "Import source": "Import source",
  "Node group import": "Node group import",
  Contents: "Contents",
  Updated: "Updated",
  "Loading proxy imports...": "Loading proxy imports...",
  "No imported sources yet. Load the global pool first.":
    "No imported sources yet. Load the global pool first.",
  "Subscription import": "Subscription import",
  "Single-node import": "Single-node import",
  "{count} proxy": "{count} proxy",
  "{count} IP": "{count} IP",
  "Updated allocation for {importId}": "Updated allocation for {importId}",
  "Deleted imported source {importId}": "Deleted imported source {importId}",
  "Proxy imports unavailable": "Proxy imports unavailable",
  "The global config can change the shared pool and profile allocations, so only admins can open it.":
    "The global config can change the shared pool and profile allocations, so only admins can open it.",
  "Import local pool for {profileId}": "Import local pool for {profileId}",
  "Import nodes for the current profile only. These nodes stay local unless you later reassign them from the inventory table.":
    "Import nodes for the current profile only. These nodes stay local unless you later reassign them from the inventory table.",
  "Import nodes for the current profile only. These nodes stay local unless you later reassign them from the global inventory.":
    "Import nodes for the current profile only. These nodes stay local unless you later reassign them from the global inventory.",
  "Import nodes for this profile only. They stay local unless you later reassign them from the global config.":
    "Import nodes for this profile only. They stay local unless you later reassign them from the global config.",
  "Import one subscription source or one node group for this profile only. They stay local unless you later reassign them from the global config.":
    "Import one subscription source or one node group for this profile only. They stay local unless you later reassign them from the global config.",
  "Only the local import and policy below are scoped to this profile.":
    "Only the local import and policy below are scoped to this profile.",
  "Scoped to {profileId} only.": "Scoped to {profileId} only.",
  "Import profile pool": "Import profile pool",
  "Import local pool": "Import local pool",
  "Profile pool updated": "Profile pool updated",
  "Local pool updated": "Local pool updated",
  "Import local proxy pool": "Import local proxy pool",
  "Imported {proxyCount} proxies across {ipCount} distinct IPs into profile {profileId}.":
    "Imported {proxyCount} proxies across {ipCount} distinct IPs into profile {profileId}.",
  "Manage local imports and whether {profileId} also composes the global pool.":
    "Manage local imports and whether {profileId} also composes the global pool.",
  "Profile policy": "Profile policy",
  "Use global pool for {profileId}": "Use global pool for {profileId}",
  "Only changes whether {profileId} inherits the global pool.":
    "Only changes whether {profileId} inherits the global pool.",
  "Toggle whether this profile composes its effective pool from both local imports and the global pool, or only from local imports.":
    "Toggle whether this profile composes its effective pool from both local imports and the global pool, or only from local imports.",
  "global enabled": "global enabled",
  "local-only": "local-only",
  "Enabled global pool for {profileId}": "Enabled global pool for {profileId}",
  "Disabled global pool for {profileId}": "Disabled global pool for {profileId}",
  "Compose {profileId} from the global pool as well":
    "Compose {profileId} from the global pool as well",
  "Turning this off immediately rebuilds the profile from local nodes only and removes sessions that depended on global-only nodes.":
    "Turning this off immediately rebuilds the profile from local nodes only and removes sessions that depended on global-only nodes.",
  "Profile proxy settings unavailable": "Profile proxy settings unavailable",
  "Unified inventory": "Unified inventory",
  "Global pool and profile allocations": "Global pool and profile allocations",
  "Global inventory and allocations": "Global inventory and allocations",
  "Track source scope, current allocation, and where each node is effective.":
    "Track source scope, current allocation, and where each node is effective.",
  "Track source scope, current allocation, and where each imported node is effective.":
    "Track source scope, current allocation, and where each imported node is effective.",
  "See where each imported node came from, where it is allocated now, and which profiles currently inherit it.":
    "See where each imported node came from, where it is allocated now, and which profiles currently inherit it.",
  "Every imported node records both its source scope and its current allocation scope. Re-imports follow the source of truth and restore nodes that upstreams still serve.":
    "Every imported node records both its source scope and its current allocation scope. Re-imports follow the source of truth and restore nodes that upstreams still serve.",
  "{count} nodes": "{count} nodes",
  "{count} node": "{count} node",
  "current profile {profileId}": "current profile {profileId}",
  "loading inventory": "loading inventory",
  "inventory live": "inventory live",
  "Deleting or reallocating an imported node only affects the current inventory snapshot. The next source reload restores anything the upstream still contains.":
    "Deleting or reallocating an imported node only affects the current inventory snapshot. The next source reload restores anything the upstream still contains.",
  Proxy: "Proxy",
  "Source scope": "Source scope",
  "Allocation scope": "Allocation scope",
  "Effective profiles": "Effective profiles",
  "Resolved IPs": "Resolved IPs",
  Actions: "Actions",
  "Loading proxy inventory...": "Loading proxy inventory...",
  "No imported nodes yet. Import the shared global pool here, or add local nodes from a profile overview first.":
    "No imported nodes yet. Import the shared global pool here, or add local nodes from a profile overview first.",
  "No imported nodes yet. Load the global pool first.":
    "No imported nodes yet. Load the global pool first.",
  "Global pool": "Global pool",
  "No active profiles": "No active profiles",
  "+{count} more": "+{count} more",
  "No resolved IPs": "No resolved IPs",
  Delete: "Delete",
  "Deleting...": "Deleting...",
  "Updated allocation for {nodeId}": "Updated allocation for {nodeId}",
  "Deleted imported node {nodeId}": "Deleted imported node {nodeId}",
  "Proxy inventory unavailable": "Proxy inventory unavailable",
  "Cross-profile allocation and node deletion are only available after switching the current config to Global.":
    "Cross-profile allocation and node deletion are only available after switching the current config to Global.",
};
