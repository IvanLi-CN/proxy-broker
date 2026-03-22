# DB Contracts

## profile_sync_configs

- `profile_id TEXT PRIMARY KEY`
- `source_type TEXT NOT NULL`
- `source_value TEXT NOT NULL`
- `enabled INTEGER NOT NULL`
- `sync_every_sec INTEGER NOT NULL`
- `full_refresh_every_sec INTEGER NOT NULL`
- `last_sync_due_at INTEGER`
- `last_sync_started_at INTEGER`
- `last_sync_finished_at INTEGER`
- `last_full_refresh_started_at INTEGER`
- `last_full_refresh_finished_at INTEGER`
- `updated_at INTEGER NOT NULL`

## task_runs

- `run_id TEXT PRIMARY KEY`
- `profile_id TEXT NOT NULL`
- `kind TEXT NOT NULL`
- `trigger TEXT NOT NULL`
- `status TEXT NOT NULL`
- `stage TEXT NOT NULL`
- `progress_current INTEGER`
- `progress_total INTEGER`
- `started_at INTEGER`
- `finished_at INTEGER`
- `summary_json TEXT`
- `error_code TEXT`
- `error_message TEXT`

## task_run_events

- `event_id TEXT PRIMARY KEY`
- `run_id TEXT NOT NULL`
- `at INTEGER NOT NULL`
- `level TEXT NOT NULL`
- `stage TEXT NOT NULL`
- `message TEXT NOT NULL`
- `payload_json TEXT`
