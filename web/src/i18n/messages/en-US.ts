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
  "Opened {count} sessions in batch": "Opened {count} sessions in batch",
  "Copied proxy address": "Copied proxy address",
  "Could not copy proxy address": "Could not copy proxy address",
  Undo: "Undo",
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
  "Profile {profileId}": "Profile {profileId}",
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
  "Create, switch, and close sessions": "Create, switch, and close sessions",
  "Copy address format": "Copy address format",
  "SOCKS address": "SOCKS address",
  "HTTP address": "HTTP address",
  "Copy proxy address for {sessionId}": "Copy proxy address for {sessionId}",
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
  "Create node-pinned session": "Create node-pinned session",
  "Create node-pinned sessions": "Create node-pinned sessions",
  "Open one listener pinned to {proxyName}. The backend will keep the node binding fixed and use the primary resolved IP.":
    "Open one listener pinned to {proxyName}. The backend will keep the node binding fixed and use the primary resolved IP.",
  "Pick one node before opening the create-session form.":
    "Pick one node before opening the create-session form.",
  "Primary IP: {ip}": "Primary IP: {ip}",
  "Desired port (optional)": "Desired port (optional)",
  "Leave this blank to auto-assign the next available port. Set it when you need a predictable listener port.":
    "Leave this blank to auto-assign the next available port. Set it when you need a predictable listener port.",
  "Creating session...": "Creating session...",
  "Review the selected nodes before opening the batch. Each row may keep auto-assigned ports or request an explicit one.":
    "Review the selected nodes before opening the batch. Each row may keep auto-assigned ports or request an explicit one.",
  "Creating sessions...": "Creating sessions...",
  "Confirm deletion": "Confirm deletion",
  "Delete imported source {name}? This removes the whole grouped import and its child nodes from the current scope.":
    "Delete imported source {name}? This removes the whole grouped import and its child nodes from the current scope.",
  "Pick one imported source before confirming deletion.":
    "Pick one imported source before confirming deletion.",
  Cancel: "Cancel",
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
  "Current user unavailable": "Current user unavailable",
  "Admin access required": "Admin access required",
  "No active profiles": "No active profiles",
  "+{count} more": "+{count} more",
  "Subscription groups": "Subscription groups",
  "Available grouped nodes": "Available grouped nodes",
  "Grouped proxy catalog": "Grouped proxy catalog",
  "Current profile grouped nodes": "Current profile grouped nodes",
  "Proxy catalog unavailable": "Proxy catalog unavailable",
  Details: "Details",
  Status: "Status",
  "Live stream: {state}": "Live stream: {state}",
  "Select one or more nodes to run batch operations.":
    "Select one or more nodes to run batch operations.",
  "Selected {count} nodes": "Selected {count} nodes",
  "Refresh selected": "Refresh selected",
  "Probe selected": "Probe selected",
  "Create session": "Create session",
  "Create sessions": "Create sessions",
  "Open a new session from one dialog. Keep single and batch creation together, but leave the list as the default surface.":
    "Open a new session from one dialog. Keep single and batch creation together, but leave the list as the default surface.",
  "Batch create failed": "Batch create failed",
  "Current sessions": "Current sessions",
  "Session list": "Session list",
  "This list refreshes every five seconds while you stay on the route, so it reflects the backend's current session inventory.":
    "This list refreshes every five seconds while you stay on the route, so it reflects the backend's current session inventory.",
  "Keep the page focused on the current session inventory. Create new sessions or switch nodes from dialogs when you need them.":
    "Keep the page focused on the current session inventory. Create new sessions or switch nodes from dialogs when you need them.",
  "{count} sessions": "{count} sessions",
  "{count} session": "{count} session",
  "switch action in flight": "switch action in flight",
  "switch action idle": "switch action idle",
  "close action in flight": "close action in flight",
  "close action idle": "close action idle",
  "session control": "session control",
  "Polling the backend for sessions on this profile.":
    "Polling the backend for sessions on this profile.",
  "The current session list appears here as soon as the first response lands.":
    "The current session list appears here as soon as the first response lands.",
  "No sessions yet": "No sessions yet",
  "Create one session or a batch from the dialog to populate this list.":
    "Create one session or a batch from the dialog to populate this list.",
  "Edit proxy for {sessionId}": "Edit proxy for {sessionId}",
  "Switched {sessionId} to {proxyName}": "Switched {sessionId} to {proxyName}",
  "Switch session proxy": "Switch session proxy",
  "Pick a new node for {sessionId}. The session keeps the same listener and port.":
    "Pick a new node for {sessionId}. The session keeps the same listener and port.",
  "Select a session before switching its node.": "Select a session before switching its node.",
  "Listen {listen}": "Listen {listen}",
  "Selected IP {ip}": "Selected IP {ip}",
  "Filter nodes": "Filter nodes",
  "Search by node, source, IP, or location": "Search by node, source, IP, or location",
  "Sort by": "Sort by",
  "Current session last used": "Current session last used",
  "Current profile last used": "Current profile last used",
  "Loading node options…": "Loading node options…",
  "Could not load node options": "Could not load node options",
  "No matching nodes": "No matching nodes",
  Current: "Current",
  Selected: "Selected",
  "Profile last used {time}": "Profile last used {time}",
  "Probe failed": "Probe failed",
  "Switching proxy…": "Switching proxy…",
  "Use selected node": "Use selected node",
  "Switch to {proxyName} via {primaryIp}": "Switch to {proxyName} via {primaryIp}",
  "Loading grouped proxy catalog...": "Loading grouped proxy catalog...",
  "No grouped nodes yet. Import a source first.": "No grouped nodes yet. Import a source first.",
  "Select import group {name}": "Select import group {name}",
  "Select node {name}": "Select node {name}",
  "Collapse group": "Collapse group",
  "Expand group": "Expand group",
  "Batch actions stay node-scoped.": "Batch actions stay node-scoped.",
  "No geo metadata yet": "No geo metadata yet",
  "Round {round}/{total}: {latency}": "Round {round}/{total}: {latency}",
  timeout: "timeout",
  "Refreshing metadata": "Refreshing metadata",
  "Median {latency}": "Median {latency}",
  "Probe failed (0/5)": "Probe failed (0/5)",
  "No probe median yet": "No probe median yet",
  "No probe data yet": "No probe data yet",
  "Queued metadata refresh": "Queued metadata refresh",
  "Queued latency probe": "Queued latency probe",
  "Run ID: {runId}": "Run ID: {runId}",
  "Proxy metadata refresh": "Proxy metadata refresh",
  "Proxy latency probe": "Proxy latency probe",
  Operator: "Operator",
  "Listening on {listen} via {proxyName} ({selectedIp}).":
    "Listening on {listen} via {proxyName} ({selectedIp}).",
  "No resolved IPs": "No resolved IPs",
  Delete: "Delete",
  "Deleting...": "Deleting...",
  "Updated allocation for {nodeId}": "Updated allocation for {nodeId}",
  "Deleted imported node {nodeId}": "Deleted imported node {nodeId}",
  "Proxy inventory unavailable": "Proxy inventory unavailable",
  "Cross-profile allocation and node deletion are only available after switching the current config to Global.":
    "Cross-profile allocation and node deletion are only available after switching the current config to Global.",
};
