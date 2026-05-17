# Proxy Broker Probe Resource Control

## Context

Large subscription catalogs can make `proxy_latency_probe` appear network-bound while the real bottleneck is local persistence. A full probe run produces thousands of samples, and resource usage can spike when each sample causes broad SQLite reads or durable task-event writes.

## Response Pattern

- Compare observed probe duration with the concurrency and timeout model before concluding that remote nodes are slow.
- Check host threads while the run is active; `sqlx-sqlite-worker` as the top CPU thread indicates persistence/query pressure rather than Mihomo delay measurement.
- Inspect `task_runs` duration, `task_run_events` counts, `proxy_node_probe_samples` row counts, and slow SQL logs together. A healthy container can still be overloaded by durable progress bookkeeping.
- Keep per-sample UI updates on the live task stream when needed, but keep durable `task_run_events` to started, bounded batch/round progress, completed, and error summaries.

## Implementation Guardrails

- Recent probe sample reads must be bounded in the store layer. Pair-specific metadata refresh should query only the target `(node_id, ip)` and global catalog reads should apply per-pair limiting before rows leave SQLite.
- Do not reintroduce caller-side filtering over all `proxy_node_probe_samples` for a single pair.
- Batch sample persistence where possible; if live visibility requires frequent updates, emit transient SSE events separately from durable event storage.
- Retain API response shape for `recent_probe_samples`, `last_probe_samples`, and task summaries unless a separate contract change is planned.
- After deploying a persistence fix, compact historical bloat with a scoped `task_run_events` retention delete and SQLite `VACUUM`; do not treat cleanup alone as a fix.
