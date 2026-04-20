use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions};

use crate::{
    ids,
    models::{
        ApiKeyProfileScope, ApiKeyProfileScopeKind, ApiKeyRecord, IpRecord, ProbeRecord,
        ProfileProxySettings, ProxyImportRecord, ProxyImportSourceIdentity, ProxyImportSyncConfig,
        ProxyInventoryRecord, ProxyNode, ProxyScope, SessionRecord, SubscriptionSource,
        TaskEventLevel, TaskListQuery, TaskRunEventRecord, TaskRunKind, TaskRunRecord,
        TaskRunStage, TaskRunStatus, TaskRunTrigger,
    },
    store::BrokerStore,
    tasks::matches_task_query,
};

#[derive(Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("failed to create sqlite parent: {}", parent.display()))?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .with_context(|| format!("failed to open sqlite db: {}", path.display()))?;

        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS profiles (
              profile_id TEXT PRIMARY KEY,
              created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS subscription_nodes (
              profile_id TEXT NOT NULL,
              proxy_name TEXT NOT NULL,
              proxy_type TEXT NOT NULL,
              server TEXT NOT NULL,
              resolved_ips_json TEXT NOT NULL,
              raw_proxy_json TEXT NOT NULL,
              PRIMARY KEY (profile_id, proxy_name)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS ip_records (
              profile_id TEXT NOT NULL,
              ip TEXT NOT NULL,
              country_code TEXT,
              country_name TEXT,
              region_name TEXT,
              city TEXT,
              geo_source TEXT,
              probe_updated_at INTEGER,
              geo_updated_at INTEGER,
              last_used_at INTEGER,
              PRIMARY KEY (profile_id, ip)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS probe_records (
              profile_id TEXT NOT NULL,
              proxy_name TEXT NOT NULL,
              ip TEXT NOT NULL,
              target_url TEXT NOT NULL,
              ok INTEGER NOT NULL,
              latency_ms INTEGER,
              updated_at INTEGER NOT NULL,
              PRIMARY KEY (profile_id, proxy_name, ip, target_url)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        self.migrate_probe_records_schema().await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
              profile_id TEXT NOT NULL,
              session_id TEXT NOT NULL,
              listen TEXT NOT NULL,
              port INTEGER NOT NULL,
              selected_ip TEXT NOT NULL,
              proxy_name TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              PRIMARY KEY (profile_id, session_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        self.migrate_api_keys_schema().await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS profile_sync_configs (
              profile_id TEXT PRIMARY KEY,
              source_type TEXT NOT NULL,
              source_value TEXT NOT NULL,
              enabled INTEGER NOT NULL,
              sync_every_sec INTEGER NOT NULL,
              full_refresh_every_sec INTEGER NOT NULL,
              last_sync_due_at INTEGER,
              last_sync_started_at INTEGER,
              last_sync_finished_at INTEGER,
              last_full_refresh_due_at INTEGER,
              last_full_refresh_started_at INTEGER,
              last_full_refresh_finished_at INTEGER,
              updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS proxy_imports (
              import_id TEXT PRIMARY KEY,
              name TEXT,
              import_kind TEXT NOT NULL,
              source_scope_type TEXT NOT NULL,
              source_scope_profile_id TEXT,
              source_type TEXT NOT NULL,
              source_value TEXT NOT NULL,
              allocation_scope_type TEXT NOT NULL,
              allocation_scope_profile_id TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_proxy_imports_source_scope
            ON proxy_imports(source_scope_type, source_scope_profile_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_proxy_imports_allocation_scope
            ON proxy_imports(allocation_scope_type, allocation_scope_profile_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS proxy_import_sync_configs (
              import_id TEXT PRIMARY KEY,
              profile_id TEXT NOT NULL,
              source_type TEXT NOT NULL,
              source_value TEXT NOT NULL,
              enabled INTEGER NOT NULL,
              sync_every_sec INTEGER NOT NULL,
              full_refresh_every_sec INTEGER NOT NULL,
              last_sync_due_at INTEGER,
              last_sync_started_at INTEGER,
              last_sync_finished_at INTEGER,
              last_full_refresh_due_at INTEGER,
              last_full_refresh_started_at INTEGER,
              last_full_refresh_finished_at INTEGER,
              updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_proxy_import_sync_configs_profile
            ON proxy_import_sync_configs(profile_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS proxy_inventory_nodes (
              node_id TEXT PRIMARY KEY,
              import_id TEXT NOT NULL,
              source_scope_type TEXT NOT NULL,
              source_scope_profile_id TEXT,
              source_type TEXT NOT NULL DEFAULT 'legacy',
              source_value TEXT NOT NULL DEFAULT '',
              allocation_scope_type TEXT NOT NULL,
              allocation_scope_profile_id TEXT,
              proxy_name TEXT NOT NULL,
              proxy_type TEXT NOT NULL,
              server TEXT NOT NULL,
              resolved_ips_json TEXT NOT NULL,
              raw_proxy_json TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Legacy databases may already have proxy_inventory_nodes without the
        // import-level columns. Repair that schema before creating any indexes
        // that depend on import_id/source_type/source_value.
        self.migrate_proxy_inventory_import_schema().await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_proxy_inventory_import
            ON proxy_inventory_nodes(import_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_proxy_inventory_source_scope
            ON proxy_inventory_nodes(source_scope_type, source_scope_profile_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_proxy_inventory_allocation_scope
            ON proxy_inventory_nodes(allocation_scope_type, allocation_scope_profile_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS profile_proxy_settings (
              profile_id TEXT PRIMARY KEY,
              use_global_proxies INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS task_runs (
              run_id TEXT PRIMARY KEY,
              profile_id TEXT NOT NULL,
              kind TEXT NOT NULL,
              trigger TEXT NOT NULL,
              status TEXT NOT NULL,
              stage TEXT NOT NULL,
              progress_current INTEGER,
              progress_total INTEGER,
              created_at INTEGER NOT NULL,
              started_at INTEGER,
              finished_at INTEGER,
              summary_json TEXT,
              error_code TEXT,
              error_message TEXT,
              scope_json TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_task_runs_profile_created
            ON task_runs(profile_id, created_at DESC, run_id DESC)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS task_run_events (
              event_id TEXT PRIMARY KEY,
              run_id TEXT NOT NULL,
              profile_id TEXT NOT NULL,
              at INTEGER NOT NULL,
              level TEXT NOT NULL,
              stage TEXT NOT NULL,
              message TEXT NOT NULL,
              payload_json TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_task_run_events_run
            ON task_run_events(run_id, at ASC, event_id ASC)
            "#,
        )
        .execute(&self.pool)
        .await?;

        self.migrate_proxy_import_sync_configs().await?;
        self.backfill_proxy_imports_from_inventory().await?;
        self.migrate_short_ids().await?;

        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_proxy_inventory_import_proxy_name
            ON proxy_inventory_nodes(import_id, proxy_name)
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn migrate_short_ids(&self) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        #[derive(Clone)]
        struct SyncTargetImport {
            target_import_id: String,
            source_identity: ProxyImportSourceIdentity,
        }

        #[derive(Clone)]
        struct PlannedImportMigration {
            original_import_id: String,
            next_record: ProxyImportRecord,
        }

        #[derive(Clone)]
        struct PlannedInventoryMigration {
            original_node_id: String,
            next_record: ProxyInventoryRecord,
        }

        let sync_config_rows = sqlx::query(
            r#"
            SELECT import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
                   last_sync_due_at, last_sync_started_at, last_sync_finished_at,
                   last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
                   updated_at
            FROM proxy_import_sync_configs
            ORDER BY profile_id ASC, import_id ASC
            "#,
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut sync_target_import_ids = HashMap::new();
        for row in sync_config_rows {
            let config = map_proxy_import_sync_config_row(row)?;
            let source_identity = ProxyImportSourceIdentity::from_source(&config.source);
            let target_import_id = ids::stable_import_id(
                &ProxyScope::profile(&config.profile_id).key(),
                &source_identity.key(),
            );
            sync_target_import_ids.insert(
                config.import_id,
                SyncTargetImport {
                    target_import_id,
                    source_identity,
                },
            );
        }

        let import_rows = sqlx::query(
            r#"
            SELECT import_id, name, import_kind, source_scope_type, source_scope_profile_id,
                   source_type, source_value, allocation_scope_type, allocation_scope_profile_id,
                   created_at, updated_at
            FROM proxy_imports
            ORDER BY created_at ASC, import_id ASC
            "#,
        )
        .fetch_all(&mut *tx)
        .await?;
        let import_records = import_rows
            .into_iter()
            .map(map_proxy_import_row)
            .collect::<Result<Vec<_>, _>>()?;
        let mut reserved_import_ids = HashSet::new();
        for record in &import_records {
            let is_manual = record.source_identity.source_type == "manual";
            let already_new_manual =
                ids::is_prefixed_short_id(&record.import_id, "imp", ids::ENTITY_ID_BODY_LEN)
                    && record.source_identity.source_value == record.import_id;
            if is_manual && !already_new_manual {
                continue;
            }
            let next_import_id =
                if record.import_kind == crate::models::ProxyImportKind::Subscription {
                    sync_target_import_ids
                        .get(&record.import_id)
                        .map(|target| target.target_import_id.clone())
                        .unwrap_or_else(|| {
                            ids::stable_import_id(
                                &record.source_scope.key(),
                                &record.source_identity.key(),
                            )
                        })
                } else {
                    ids::stable_import_id(&record.source_scope.key(), &record.source_identity.key())
                };
            reserved_import_ids.insert(next_import_id);
        }

        let mut import_migrations = HashMap::<String, Vec<PlannedImportMigration>>::new();
        for record in import_records {
            let is_manual = record.source_identity.source_type == "manual";
            let already_new_manual =
                ids::is_prefixed_short_id(&record.import_id, "imp", ids::ENTITY_ID_BODY_LEN)
                    && record.source_identity.source_value == record.import_id;
            let (next_import_id, next_source_identity) = if is_manual {
                if already_new_manual {
                    (record.import_id.clone(), record.source_identity.clone())
                } else {
                    (
                        reserve_unique_id(&mut reserved_import_ids, ids::random_import_id),
                        ProxyImportSourceIdentity {
                            source_type: "manual".to_string(),
                            source_value: String::new(),
                        },
                    )
                }
            } else if record.import_kind == crate::models::ProxyImportKind::Subscription {
                if let Some(target) = sync_target_import_ids.get(&record.import_id) {
                    (
                        target.target_import_id.clone(),
                        target.source_identity.clone(),
                    )
                } else {
                    (
                        ids::stable_import_id(
                            &record.source_scope.key(),
                            &record.source_identity.key(),
                        ),
                        record.source_identity.clone(),
                    )
                }
            } else {
                (
                    ids::stable_import_id(
                        &record.source_scope.key(),
                        &record.source_identity.key(),
                    ),
                    record.source_identity.clone(),
                )
            };
            let mut next_record = record.clone();
            next_record.import_id = next_import_id.clone();
            next_record.source_identity = next_source_identity;
            if is_manual {
                next_record.source_identity.source_value = next_import_id.clone();
            }
            import_migrations
                .entry(next_import_id)
                .or_default()
                .push(PlannedImportMigration {
                    original_import_id: record.import_id,
                    next_record,
                });
        }

        let mut import_id_rewrites = HashMap::new();
        for (target_import_id, plans) in import_migrations {
            let mut merged = plans[0].next_record.clone();
            for plan in plans.iter().skip(1) {
                merged = merge_proxy_import_records(&merged, &plan.next_record, &target_import_id);
            }

            persist_proxy_import(&mut tx, &merged).await?;

            for plan in plans {
                import_id_rewrites
                    .insert(plan.original_import_id.clone(), target_import_id.clone());
                if plan.original_import_id == target_import_id {
                    continue;
                }

                sqlx::query("DELETE FROM proxy_imports WHERE import_id = ?1")
                    .bind(&plan.original_import_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }

        let sync_rows = sqlx::query(
            r#"
            SELECT import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
                   last_sync_due_at, last_sync_started_at, last_sync_finished_at,
                   last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
                   updated_at
            FROM proxy_import_sync_configs
            ORDER BY profile_id ASC, import_id ASC
            "#,
        )
        .fetch_all(&mut *tx)
        .await?;
        for row in sync_rows {
            let config = map_proxy_import_sync_config_row(row)?;
            let source_identity = ProxyImportSourceIdentity::from_source(&config.source);
            let next_import_id = ids::stable_import_id(
                &ProxyScope::profile(&config.profile_id).key(),
                &source_identity.key(),
            );
            if next_import_id == config.import_id {
                continue;
            }
            let existing = sqlx::query(
                r#"
                SELECT import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
                       last_sync_due_at, last_sync_started_at, last_sync_finished_at,
                       last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
                       updated_at
                FROM proxy_import_sync_configs
                WHERE import_id = ?1
                "#,
            )
            .bind(&next_import_id)
            .fetch_optional(&mut *tx)
            .await?
            .map(map_proxy_import_sync_config_row)
            .transpose()?;

            if let Some(existing) = existing {
                let merged = merge_proxy_import_sync_configs(&existing, &config, &next_import_id);
                let (source_type, source_value) = merged.source.parts();
                sqlx::query(
                    r#"
                    UPDATE proxy_import_sync_configs
                    SET profile_id = ?2,
                        source_type = ?3,
                        source_value = ?4,
                        enabled = ?5,
                        sync_every_sec = ?6,
                        full_refresh_every_sec = ?7,
                        last_sync_due_at = ?8,
                        last_sync_started_at = ?9,
                        last_sync_finished_at = ?10,
                        last_full_refresh_due_at = ?11,
                        last_full_refresh_started_at = ?12,
                        last_full_refresh_finished_at = ?13,
                        updated_at = ?14
                    WHERE import_id = ?1
                    "#,
                )
                .bind(&merged.import_id)
                .bind(&merged.profile_id)
                .bind(source_type)
                .bind(source_value)
                .bind(merged.enabled as i64)
                .bind(merged.sync_every_sec as i64)
                .bind(merged.full_refresh_every_sec as i64)
                .bind(merged.last_sync_due_at)
                .bind(merged.last_sync_started_at)
                .bind(merged.last_sync_finished_at)
                .bind(merged.last_full_refresh_due_at)
                .bind(merged.last_full_refresh_started_at)
                .bind(merged.last_full_refresh_finished_at)
                .bind(merged.updated_at)
                .execute(&mut *tx)
                .await?;
                sqlx::query("DELETE FROM proxy_import_sync_configs WHERE import_id = ?1")
                    .bind(&config.import_id)
                    .execute(&mut *tx)
                    .await?;
            } else {
                sqlx::query(
                    "UPDATE proxy_import_sync_configs SET import_id = ?1 WHERE import_id = ?2",
                )
                .bind(&next_import_id)
                .bind(&config.import_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        let inventory_rows = sqlx::query(
            r#"
            SELECT import_id, node_id, source_scope_type, source_scope_profile_id, source_type, source_value,
                   allocation_scope_type, allocation_scope_profile_id,
                   proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json,
                   created_at, updated_at
            FROM proxy_inventory_nodes
            ORDER BY created_at ASC, node_id ASC
            "#,
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut inventory_migrations = HashMap::<String, Vec<PlannedInventoryMigration>>::new();
        for row in inventory_rows {
            let record = map_proxy_inventory_row(row)?;
            let next_import_id = import_id_rewrites
                .get(&record.import_id)
                .cloned()
                .unwrap_or_else(|| record.import_id.clone());
            let next_node_id =
                ids::stable_proxy_inventory_node_id(&next_import_id, &record.proxy_name);
            let mut next_record = record.clone();
            next_record.import_id = next_import_id;
            next_record.node_id = next_node_id.clone();
            inventory_migrations
                .entry(next_node_id)
                .or_default()
                .push(PlannedInventoryMigration {
                    original_node_id: record.node_id,
                    next_record,
                });
        }

        for (target_node_id, plans) in inventory_migrations {
            let mut merged = plans[0].next_record.clone();
            for plan in plans.iter().skip(1) {
                merged = merge_proxy_inventory_records(&merged, &plan.next_record, &target_node_id);
            }

            let keeper_node_id = plans
                .iter()
                .find(|plan| plan.original_node_id == target_node_id)
                .map(|plan| plan.original_node_id.clone())
                .unwrap_or_else(|| plans[0].original_node_id.clone());

            for plan in &plans {
                if plan.original_node_id == keeper_node_id {
                    continue;
                }
                sqlx::query("DELETE FROM proxy_inventory_nodes WHERE node_id = ?1")
                    .bind(&plan.original_node_id)
                    .execute(&mut *tx)
                    .await?;
            }

            sqlx::query(
                r#"
                UPDATE proxy_inventory_nodes
                SET import_id = ?1,
                    node_id = ?2,
                    source_scope_type = ?3,
                    source_scope_profile_id = ?4,
                    source_type = 'inventory',
                    source_value = ?1,
                    allocation_scope_type = ?5,
                    allocation_scope_profile_id = ?6,
                    proxy_name = ?7,
                    proxy_type = ?8,
                    server = ?9,
                    resolved_ips_json = ?10,
                    raw_proxy_json = ?11,
                    created_at = ?12,
                    updated_at = ?13
                WHERE node_id = ?14
                "#,
            )
            .bind(&merged.import_id)
            .bind(&target_node_id)
            .bind(merged.source_scope.kind())
            .bind(merged.source_scope.profile_id())
            .bind(merged.allocation_scope.kind())
            .bind(merged.allocation_scope.profile_id())
            .bind(&merged.proxy_name)
            .bind(&merged.proxy_type)
            .bind(&merged.server)
            .bind(serde_json::to_string(&merged.resolved_ips)?)
            .bind(serde_json::to_string(&merged.raw_proxy)?)
            .bind(merged.created_at)
            .bind(merged.updated_at)
            .bind(&keeper_node_id)
            .execute(&mut *tx)
            .await?;
        }

        let session_rows = sqlx::query(
            "SELECT profile_id, session_id FROM sessions ORDER BY created_at ASC, session_id ASC",
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut reserved_session_ids = HashSet::new();
        for row in session_rows {
            let profile_id: String = row.try_get("profile_id")?;
            let session_id: String = row.try_get("session_id")?;
            let next_session_id =
                if ids::is_prefixed_short_id(&session_id, "sess", ids::ENTITY_ID_BODY_LEN) {
                    session_id.clone()
                } else {
                    reserve_unique_id(&mut reserved_session_ids, ids::random_session_id)
                };
            anyhow::ensure!(
                reserved_session_ids.insert(next_session_id.clone()),
                "session short-id migration collision for {}",
                session_id
            );
            if next_session_id == session_id {
                continue;
            }
            sqlx::query(
                "UPDATE sessions SET session_id = ?1 WHERE profile_id = ?2 AND session_id = ?3",
            )
            .bind(&next_session_id)
            .bind(&profile_id)
            .bind(&session_id)
            .execute(&mut *tx)
            .await?;
        }

        let run_rows =
            sqlx::query("SELECT run_id FROM task_runs ORDER BY created_at ASC, run_id ASC")
                .fetch_all(&mut *tx)
                .await?;
        let mut reserved_run_ids = HashSet::new();
        let mut run_id_updates = Vec::new();
        for row in run_rows {
            let run_id: String = row.try_get("run_id")?;
            let next_run_id = if ids::is_prefixed_short_id(&run_id, "run", ids::ENTITY_ID_BODY_LEN)
            {
                run_id.clone()
            } else {
                reserve_unique_id(&mut reserved_run_ids, ids::random_task_run_id)
            };
            anyhow::ensure!(
                reserved_run_ids.insert(next_run_id.clone()),
                "task run short-id migration collision for {}",
                run_id
            );
            if next_run_id != run_id {
                run_id_updates.push((run_id, next_run_id));
            }
        }
        for (run_id, next_run_id) in &run_id_updates {
            sqlx::query("UPDATE task_runs SET run_id = ?1 WHERE run_id = ?2")
                .bind(next_run_id)
                .bind(run_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE task_run_events SET run_id = ?1 WHERE run_id = ?2")
                .bind(next_run_id)
                .bind(run_id)
                .execute(&mut *tx)
                .await?;
        }

        let event_rows =
            sqlx::query("SELECT event_id FROM task_run_events ORDER BY at ASC, event_id ASC")
                .fetch_all(&mut *tx)
                .await?;
        let mut reserved_event_ids = HashSet::new();
        for row in event_rows {
            let event_id: String = row.try_get("event_id")?;
            let next_event_id =
                if ids::is_prefixed_short_id(&event_id, "evt", ids::ENTITY_ID_BODY_LEN) {
                    event_id.clone()
                } else {
                    reserve_unique_id(&mut reserved_event_ids, ids::random_task_event_id)
                };
            anyhow::ensure!(
                reserved_event_ids.insert(next_event_id.clone()),
                "task event short-id migration collision for {}",
                event_id
            );
            if next_event_id == event_id {
                continue;
            }
            sqlx::query("UPDATE task_run_events SET event_id = ?1 WHERE event_id = ?2")
                .bind(&next_event_id)
                .bind(&event_id)
                .execute(&mut *tx)
                .await?;
        }

        let api_key_rows = sqlx::query("SELECT key_id FROM api_keys")
            .fetch_all(&mut *tx)
            .await?;
        for row in api_key_rows {
            let key_id: String = row.try_get("key_id")?;
            if ids::is_prefixed_short_id(&key_id, "key", ids::ENTITY_ID_BODY_LEN) {
                continue;
            }
            // API key secrets embed `key_id` inside `pbk_<key_id>_<random>`, while the database only
            // stores `hash(full_secret)` plus salt/prefix metadata. UUID-era keys therefore cannot be
            // rewritten to new short ids without reissuing the secret, so the migration intentionally
            // drops them and relies on administrators to create replacement keys after upgrade.
            sqlx::query("DELETE FROM api_key_profiles WHERE key_id = ?1")
                .bind(&key_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM api_keys WHERE key_id = ?1")
                .bind(&key_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn migrate_probe_records_schema(&self) -> anyhow::Result<()> {
        let columns = sqlx::query("PRAGMA table_info(probe_records)")
            .fetch_all(&self.pool)
            .await?;
        let has_proxy_name = columns
            .iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .any(|name| name == "proxy_name");
        if has_proxy_name {
            // Historical migrations could leave probe rows with unknown proxy_name ('').
            // They cannot be correlated to a real proxy anymore and would poison health scoring.
            sqlx::query("DELETE FROM probe_records WHERE proxy_name = ''")
                .execute(&self.pool)
                .await?;
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;
        sqlx::query("ALTER TABLE probe_records RENAME TO probe_records_old")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            r#"
            CREATE TABLE probe_records (
              profile_id TEXT NOT NULL,
              proxy_name TEXT NOT NULL,
              ip TEXT NOT NULL,
              target_url TEXT NOT NULL,
              ok INTEGER NOT NULL,
              latency_ms INTEGER,
              updated_at INTEGER NOT NULL,
              PRIMARY KEY (profile_id, proxy_name, ip, target_url)
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("DROP TABLE probe_records_old")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(())
    }

    async fn table_has_column(&self, table: &str, column: &str) -> anyhow::Result<bool> {
        let query = format!("PRAGMA table_info({table})");
        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .any(|name| name == column))
    }

    async fn migrate_proxy_inventory_import_schema(&self) -> anyhow::Result<()> {
        if !self
            .table_has_column("proxy_inventory_nodes", "import_id")
            .await?
        {
            sqlx::query(
                "ALTER TABLE proxy_inventory_nodes ADD COLUMN import_id TEXT NOT NULL DEFAULT ''",
            )
            .execute(&self.pool)
            .await?;
        }
        if !self
            .table_has_column("proxy_inventory_nodes", "source_type")
            .await?
        {
            sqlx::query(
                "ALTER TABLE proxy_inventory_nodes ADD COLUMN source_type TEXT NOT NULL DEFAULT 'legacy'",
            )
            .execute(&self.pool)
            .await?;
        }
        if !self
            .table_has_column("proxy_inventory_nodes", "source_value")
            .await?
        {
            sqlx::query(
                "ALTER TABLE proxy_inventory_nodes ADD COLUMN source_value TEXT NOT NULL DEFAULT ''",
            )
            .execute(&self.pool)
            .await?;
        }
        if !self.table_has_column("proxy_imports", "name").await? {
            sqlx::query("ALTER TABLE proxy_imports ADD COLUMN name TEXT")
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    async fn migrate_proxy_import_sync_configs(&self) -> anyhow::Result<()> {
        let legacy_rows = sqlx::query(
            r#"
            SELECT profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
                   last_sync_due_at, last_sync_started_at, last_sync_finished_at,
                   last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
                   updated_at
            FROM profile_sync_configs
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for row in legacy_rows {
            let profile_id: String = row.try_get("profile_id")?;
            let source_type: String = row.try_get("source_type")?;
            let source_value: String = row.try_get("source_value")?;
            let source_scope = ProxyScope::profile(profile_id.clone());
            let source_identity = ProxyImportSourceIdentity {
                source_type: source_type.clone(),
                source_value: source_value.clone(),
            };
            let import_id = stable_proxy_import_id(&source_scope, &source_identity);
            sqlx::query(
                r#"
                INSERT INTO proxy_import_sync_configs (
                  import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
                  last_sync_due_at, last_sync_started_at, last_sync_finished_at,
                  last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
                  updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                ON CONFLICT(import_id) DO NOTHING
                "#,
            )
            .bind(import_id)
            .bind(profile_id)
            .bind(source_type)
            .bind(source_value)
            .bind(row.try_get::<i64, _>("enabled")?)
            .bind(row.try_get::<i64, _>("sync_every_sec")?)
            .bind(row.try_get::<i64, _>("full_refresh_every_sec")?)
            .bind(row.try_get::<Option<i64>, _>("last_sync_due_at")?)
            .bind(row.try_get::<Option<i64>, _>("last_sync_started_at")?)
            .bind(row.try_get::<Option<i64>, _>("last_sync_finished_at")?)
            .bind(row.try_get::<Option<i64>, _>("last_full_refresh_due_at")?)
            .bind(row.try_get::<Option<i64>, _>("last_full_refresh_started_at")?)
            .bind(row.try_get::<Option<i64>, _>("last_full_refresh_finished_at")?)
            .bind(row.try_get::<i64, _>("updated_at")?)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn backfill_proxy_imports_from_inventory(&self) -> anyhow::Result<()> {
        let rows = sqlx::query(
            r#"
            SELECT node_id, import_id, source_scope_type, source_scope_profile_id, source_type, source_value,
                   allocation_scope_type, allocation_scope_profile_id, proxy_name, proxy_type, server,
                   resolved_ips_json, raw_proxy_json, created_at, updated_at
            FROM proxy_inventory_nodes
            ORDER BY created_at ASC, node_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        if rows.is_empty() {
            return Ok(());
        }

        let legacy_configs =
            sqlx::query("SELECT profile_id, source_type, source_value FROM profile_sync_configs")
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|row| {
                    Ok::<_, anyhow::Error>((
                        row.try_get::<String, _>("profile_id")?,
                        ProxyImportSourceIdentity {
                            source_type: row.try_get("source_type")?,
                            source_value: row.try_get("source_value")?,
                        },
                    ))
                })
                .collect::<Result<std::collections::HashMap<_, _>, _>>()?;

        #[derive(Clone)]
        struct ImportSeed {
            record: ProxyImportRecord,
        }

        let mut imports = std::collections::HashMap::<String, ImportSeed>::new();
        let mut tx = self.pool.begin().await?;
        for row in rows {
            let source_scope = ProxyScope::from_parts(
                &row.try_get::<String, _>("source_scope_type")?,
                row.try_get("source_scope_profile_id")?,
            )
            .ok_or_else(|| anyhow!("invalid proxy inventory source scope"))?;
            let allocation_scope = ProxyScope::from_parts(
                &row.try_get::<String, _>("allocation_scope_type")?,
                row.try_get("allocation_scope_profile_id")?,
            )
            .ok_or_else(|| anyhow!("invalid proxy inventory allocation scope"))?;

            let source_identity = {
                let source_type: String = row.try_get("source_type")?;
                let source_value: String = row.try_get("source_value")?;
                if !(source_type.trim().is_empty()
                    || (source_type == "legacy" && source_value.is_empty()))
                {
                    ProxyImportSourceIdentity {
                        source_type,
                        source_value,
                    }
                } else if let Some(profile_id) = source_scope.profile_id() {
                    legacy_configs
                        .get(profile_id)
                        .cloned()
                        .unwrap_or(ProxyImportSourceIdentity {
                            source_type: "legacy_scope".to_string(),
                            source_value: source_scope.key(),
                        })
                } else {
                    ProxyImportSourceIdentity {
                        source_type: "legacy_scope".to_string(),
                        source_value: source_scope.key(),
                    }
                }
            };

            let import_id: String = {
                let existing: String = row.try_get("import_id")?;
                if existing.trim().is_empty() {
                    stable_proxy_import_id(&source_scope, &source_identity)
                } else {
                    existing
                }
            };

            sqlx::query(
                "UPDATE proxy_inventory_nodes SET import_id = ?2, source_type = ?3, source_value = ?4 WHERE node_id = ?1",
            )
            .bind(row.try_get::<String, _>("node_id")?)
            .bind(&import_id)
            .bind(&source_identity.source_type)
            .bind(&source_identity.source_value)
            .execute(&mut *tx)
            .await?;

            let created_at: i64 = row.try_get("created_at")?;
            let updated_at: i64 = row.try_get("updated_at")?;
            imports
                .entry(import_id.clone())
                .and_modify(|seed| {
                    if updated_at >= seed.record.updated_at {
                        seed.record.allocation_scope = allocation_scope.clone();
                        seed.record.updated_at = updated_at;
                    }
                    if created_at < seed.record.created_at {
                        seed.record.created_at = created_at;
                    }
                })
                .or_insert_with(|| ImportSeed {
                    record: ProxyImportRecord {
                        import_id: import_id.clone(),
                        name: None,
                        import_kind: crate::models::ProxyImportKind::Subscription,
                        source_scope: source_scope.clone(),
                        source_identity: source_identity.clone(),
                        allocation_scope: allocation_scope.clone(),
                        created_at,
                        updated_at,
                    },
                });
        }

        for seed in imports.into_values() {
            persist_proxy_import(&mut tx, &seed.record).await?;
            sqlx::query(
                "UPDATE proxy_inventory_nodes SET allocation_scope_type = ?2, allocation_scope_profile_id = ?3 WHERE import_id = ?1",
            )
            .bind(&seed.record.import_id)
            .bind(seed.record.allocation_scope.kind())
            .bind(seed.record.allocation_scope.profile_id())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn migrate_api_keys_schema(&self) -> anyhow::Result<()> {
        let columns = sqlx::query("PRAGMA table_info(api_keys)")
            .fetch_all(&self.pool)
            .await?;
        let has_scope_kind = columns
            .iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .any(|name| name == "scope_kind");
        let has_profile_id = columns
            .iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .any(|name| name == "profile_id");

        if columns.is_empty() {
            sqlx::query(
                r#"
                CREATE TABLE api_keys (
                  key_id TEXT PRIMARY KEY,
                  name TEXT NOT NULL,
                  secret_prefix TEXT NOT NULL,
                  secret_salt TEXT NOT NULL,
                  secret_hash TEXT NOT NULL,
                  created_by_subject TEXT NOT NULL,
                  scope_kind TEXT NOT NULL,
                  created_at INTEGER NOT NULL,
                  last_used_at INTEGER,
                  revoked_at INTEGER
                )
                "#,
            )
            .execute(&self.pool)
            .await?;
        } else if has_profile_id || !has_scope_kind {
            let mut tx = self.pool.begin().await?;
            sqlx::query("ALTER TABLE api_keys RENAME TO api_keys_old")
                .execute(&mut *tx)
                .await?;

            sqlx::query(
                r#"
                CREATE TABLE api_keys (
                  key_id TEXT PRIMARY KEY,
                  name TEXT NOT NULL,
                  secret_prefix TEXT NOT NULL,
                  secret_salt TEXT NOT NULL,
                  secret_hash TEXT NOT NULL,
                  created_by_subject TEXT NOT NULL,
                  scope_kind TEXT NOT NULL,
                  created_at INTEGER NOT NULL,
                  last_used_at INTEGER,
                  revoked_at INTEGER
                )
                "#,
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO api_keys (
                  key_id, name, secret_prefix, secret_salt, secret_hash,
                  created_by_subject, scope_kind, created_at, last_used_at, revoked_at
                )
                SELECT
                  key_id, name, secret_prefix, secret_salt, secret_hash,
                  created_by_subject, 'selected_profiles', created_at, last_used_at, revoked_at
                FROM api_keys_old
                "#,
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS api_key_profiles (
                  key_id TEXT NOT NULL,
                  profile_id TEXT NOT NULL,
                  PRIMARY KEY (key_id, profile_id)
                )
                "#,
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO api_key_profiles (key_id, profile_id)
                SELECT key_id, profile_id
                FROM api_keys_old
                "#,
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query("DROP TABLE api_keys_old")
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_key_profiles (
              key_id TEXT NOT NULL,
              profile_id TEXT NOT NULL,
              PRIMARY KEY (key_id, profile_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_api_key_profiles_profile
            ON api_key_profiles(profile_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_api_keys_secret_hash
            ON api_keys(secret_hash)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl BrokerStore for SqliteStore {
    async fn list_profiles(&self) -> anyhow::Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT profile_id
            FROM (
              SELECT profile_id FROM profiles
              UNION
              SELECT profile_id FROM subscription_nodes
              UNION
              SELECT profile_id FROM ip_records
              UNION
              SELECT profile_id FROM probe_records
              UNION
              SELECT profile_id FROM sessions
              UNION
              SELECT profile_id FROM api_key_profiles
              UNION
              SELECT profile_id FROM profile_sync_configs
              UNION
              SELECT profile_id FROM proxy_import_sync_configs
              UNION
              SELECT profile_id FROM profile_proxy_settings
              UNION
              SELECT source_scope_profile_id AS profile_id
              FROM proxy_inventory_nodes
              WHERE source_scope_type = 'profile' AND source_scope_profile_id IS NOT NULL
              UNION
              SELECT allocation_scope_profile_id AS profile_id
              FROM proxy_inventory_nodes
              WHERE allocation_scope_type = 'profile' AND allocation_scope_profile_id IS NOT NULL
            )
            WHERE profile_id IS NOT NULL
            ORDER BY profile_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| row.try_get("profile_id").map_err(anyhow::Error::from))
            .collect()
    }

    async fn create_profile(&self, profile_id: &str, created_at: i64) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            INSERT INTO profiles (profile_id, created_at)
            VALUES (?1, ?2)
            "#,
        )
        .bind(profile_id)
        .bind(created_at)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO profile_proxy_settings (profile_id, use_global_proxies, updated_at)
            VALUES (?1, 1, ?2)
            ON CONFLICT(profile_id) DO NOTHING
            "#,
        )
        .bind(profile_id)
        .bind(created_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn replace_subscription(
        &self,
        profile_id: &str,
        nodes: &[ProxyNode],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM subscription_nodes WHERE profile_id = ?1")
            .bind(profile_id)
            .execute(&mut *tx)
            .await?;
        for node in nodes {
            sqlx::query(
                r#"
                INSERT INTO subscription_nodes (
                  profile_id, proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
            )
            .bind(profile_id)
            .bind(&node.proxy_name)
            .bind(&node.proxy_type)
            .bind(&node.server)
            .bind(serde_json::to_string(&node.resolved_ips)?)
            .bind(serde_json::to_string(&node.raw_proxy)?)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn apply_subscription_snapshot(
        &self,
        profile_id: &str,
        nodes: &[ProxyNode],
        ip_records: &[IpRecord],
        probe_records: &[ProbeRecord],
        removed_session_ids: &[String],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM subscription_nodes WHERE profile_id = ?1")
            .bind(profile_id)
            .execute(&mut *tx)
            .await?;
        for node in nodes {
            sqlx::query(
                r#"
                INSERT INTO subscription_nodes (
                  profile_id, proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
            )
            .bind(profile_id)
            .bind(&node.proxy_name)
            .bind(&node.proxy_type)
            .bind(&node.server)
            .bind(serde_json::to_string(&node.resolved_ips)?)
            .bind(serde_json::to_string(&node.raw_proxy)?)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query("DELETE FROM ip_records WHERE profile_id = ?1")
            .bind(profile_id)
            .execute(&mut *tx)
            .await?;
        for record in ip_records {
            sqlx::query(
                r#"
                INSERT INTO ip_records (
                  profile_id, ip, country_code, country_name, region_name, city,
                  geo_source, probe_updated_at, geo_updated_at, last_used_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
            )
            .bind(profile_id)
            .bind(&record.ip)
            .bind(&record.country_code)
            .bind(&record.country_name)
            .bind(&record.region_name)
            .bind(&record.city)
            .bind(&record.geo_source)
            .bind(record.probe_updated_at)
            .bind(record.geo_updated_at)
            .bind(record.last_used_at)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query("DELETE FROM probe_records WHERE profile_id = ?1")
            .bind(profile_id)
            .execute(&mut *tx)
            .await?;
        for record in probe_records {
            sqlx::query(
                r#"
                INSERT INTO probe_records (
                  profile_id, proxy_name, ip, target_url, ok, latency_ms, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
            )
            .bind(profile_id)
            .bind(&record.proxy_name)
            .bind(&record.ip)
            .bind(&record.target_url)
            .bind(record.ok as i64)
            .bind(record.latency_ms.map(|x| x as i64))
            .bind(record.updated_at)
            .execute(&mut *tx)
            .await?;
        }

        for session_id in removed_session_ids {
            sqlx::query("DELETE FROM sessions WHERE profile_id = ?1 AND session_id = ?2")
                .bind(profile_id)
                .bind(session_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn list_subscription(&self, profile_id: &str) -> anyhow::Result<Vec<ProxyNode>> {
        let rows = sqlx::query(
            r#"
            SELECT proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json
            FROM subscription_nodes
            WHERE profile_id = ?1
            ORDER BY proxy_name
            "#,
        )
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let resolved_ips_json: String = row.try_get("resolved_ips_json")?;
                let raw_proxy_json: String = row.try_get("raw_proxy_json")?;
                Ok(ProxyNode {
                    proxy_name: row.try_get("proxy_name")?,
                    proxy_type: row.try_get("proxy_type")?,
                    server: row.try_get("server")?,
                    resolved_ips: serde_json::from_str(&resolved_ips_json)?,
                    raw_proxy: serde_json::from_str(&raw_proxy_json)?,
                })
            })
            .collect()
    }

    async fn list_proxy_inventory(&self) -> anyhow::Result<Vec<ProxyInventoryRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT import_id, node_id, source_scope_type, source_scope_profile_id, source_type, source_value,
                   allocation_scope_type, allocation_scope_profile_id,
                   proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json,
                   created_at, updated_at
            FROM proxy_inventory_nodes
            ORDER BY proxy_name ASC, node_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_proxy_inventory_row).collect()
    }

    async fn replace_proxy_inventory_scope(
        &self,
        source_scope: &ProxyScope,
        nodes: &[ProxyInventoryRecord],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            DELETE FROM proxy_inventory_nodes
            WHERE source_scope_type = ?1 AND (
              (?2 IS NULL AND source_scope_profile_id IS NULL) OR source_scope_profile_id = ?2
            )
            "#,
        )
        .bind(source_scope.kind())
        .bind(source_scope.profile_id())
        .execute(&mut *tx)
        .await?;

        for node in nodes {
            persist_proxy_inventory_node(&mut tx, node).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_proxy_imports(&self) -> anyhow::Result<Vec<ProxyImportRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT import_id, name, import_kind, source_scope_type, source_scope_profile_id,
                   source_type, source_value, allocation_scope_type, allocation_scope_profile_id,
                   created_at, updated_at
            FROM proxy_imports
            ORDER BY created_at ASC, import_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_proxy_import_row).collect()
    }

    async fn get_proxy_import(&self, import_id: &str) -> anyhow::Result<Option<ProxyImportRecord>> {
        let row = sqlx::query(
            r#"
            SELECT import_id, name, import_kind, source_scope_type, source_scope_profile_id,
                   source_type, source_value, allocation_scope_type, allocation_scope_profile_id,
                   created_at, updated_at
            FROM proxy_imports
            WHERE import_id = ?1
            "#,
        )
        .bind(import_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_proxy_import_row).transpose()
    }

    async fn replace_proxy_inventory_import(
        &self,
        import_record: &ProxyImportRecord,
        nodes: &[ProxyInventoryRecord],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        persist_proxy_import(&mut tx, import_record).await?;
        sqlx::query("DELETE FROM proxy_inventory_nodes WHERE import_id = ?1")
            .bind(&import_record.import_id)
            .execute(&mut *tx)
            .await?;

        for node in nodes {
            persist_proxy_inventory_node(&mut tx, node).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn get_proxy_inventory_node(
        &self,
        node_id: &str,
    ) -> anyhow::Result<Option<ProxyInventoryRecord>> {
        let row = sqlx::query(
            r#"
            SELECT import_id, node_id, source_scope_type, source_scope_profile_id, source_type, source_value,
                   allocation_scope_type, allocation_scope_profile_id,
                   proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json,
                   created_at, updated_at
            FROM proxy_inventory_nodes
            WHERE node_id = ?1
            "#,
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_proxy_inventory_row).transpose()
    }

    async fn list_proxy_inventory_for_import(
        &self,
        import_id: &str,
    ) -> anyhow::Result<Vec<ProxyInventoryRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT import_id, node_id, source_scope_type, source_scope_profile_id, source_type, source_value,
                   allocation_scope_type, allocation_scope_profile_id,
                   proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json,
                   created_at, updated_at
            FROM proxy_inventory_nodes
            WHERE import_id = ?1
            ORDER BY proxy_name ASC, node_id ASC
            "#,
        )
        .bind(import_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_proxy_inventory_row).collect()
    }

    async fn update_proxy_inventory_allocation(
        &self,
        node_id: &str,
        allocation_scope: &ProxyScope,
        updated_at: i64,
    ) -> anyhow::Result<Option<ProxyInventoryRecord>> {
        let result = sqlx::query(
            r#"
            UPDATE proxy_inventory_nodes
            SET allocation_scope_type = ?2,
                allocation_scope_profile_id = ?3,
                updated_at = ?4
            WHERE node_id = ?1
            "#,
        )
        .bind(node_id)
        .bind(allocation_scope.kind())
        .bind(allocation_scope.profile_id())
        .bind(updated_at)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        self.get_proxy_inventory_node(node_id).await
    }

    async fn update_proxy_import_allocation(
        &self,
        import_id: &str,
        allocation_scope: &ProxyScope,
        updated_at: i64,
    ) -> anyhow::Result<Option<ProxyImportRecord>> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            r#"
            UPDATE proxy_imports
            SET allocation_scope_type = ?2,
                allocation_scope_profile_id = ?3,
                updated_at = ?4
            WHERE import_id = ?1
            "#,
        )
        .bind(import_id)
        .bind(allocation_scope.kind())
        .bind(allocation_scope.profile_id())
        .bind(updated_at)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(None);
        }

        sqlx::query(
            r#"
            UPDATE proxy_inventory_nodes
            SET allocation_scope_type = ?2,
                allocation_scope_profile_id = ?3,
                updated_at = ?4
            WHERE import_id = ?1
            "#,
        )
        .bind(import_id)
        .bind(allocation_scope.kind())
        .bind(allocation_scope.profile_id())
        .bind(updated_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.get_proxy_import(import_id).await
    }

    async fn delete_proxy_inventory_node(
        &self,
        node_id: &str,
    ) -> anyhow::Result<Option<ProxyInventoryRecord>> {
        let existing = self.get_proxy_inventory_node(node_id).await?;
        if existing.is_none() {
            return Ok(None);
        }
        sqlx::query("DELETE FROM proxy_inventory_nodes WHERE node_id = ?1")
            .bind(node_id)
            .execute(&self.pool)
            .await?;
        Ok(existing)
    }

    async fn delete_proxy_import(
        &self,
        import_id: &str,
    ) -> anyhow::Result<Option<ProxyImportRecord>> {
        let existing = self.get_proxy_import(import_id).await?;
        if existing.is_none() {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM proxy_inventory_nodes WHERE import_id = ?1")
            .bind(import_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM proxy_import_sync_configs WHERE import_id = ?1")
            .bind(import_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM proxy_imports WHERE import_id = ?1")
            .bind(import_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(existing)
    }

    async fn replace_ip_records(
        &self,
        profile_id: &str,
        records: &[IpRecord],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM ip_records WHERE profile_id = ?1")
            .bind(profile_id)
            .execute(&mut *tx)
            .await?;

        for record in records {
            sqlx::query(
                r#"
                INSERT INTO ip_records (
                  profile_id, ip, country_code, country_name, region_name, city,
                  geo_source, probe_updated_at, geo_updated_at, last_used_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
            )
            .bind(profile_id)
            .bind(&record.ip)
            .bind(&record.country_code)
            .bind(&record.country_name)
            .bind(&record.region_name)
            .bind(&record.city)
            .bind(&record.geo_source)
            .bind(record.probe_updated_at)
            .bind(record.geo_updated_at)
            .bind(record.last_used_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn upsert_ip_records(
        &self,
        profile_id: &str,
        records: &[IpRecord],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        for record in records {
            sqlx::query(
                r#"
                INSERT INTO ip_records (
                  profile_id, ip, country_code, country_name, region_name, city,
                  geo_source, probe_updated_at, geo_updated_at, last_used_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT(profile_id, ip) DO UPDATE SET
                  country_code = excluded.country_code,
                  country_name = excluded.country_name,
                  region_name = excluded.region_name,
                  city = excluded.city,
                  geo_source = excluded.geo_source,
                  probe_updated_at = excluded.probe_updated_at,
                  geo_updated_at = excluded.geo_updated_at,
                  last_used_at = excluded.last_used_at
                "#,
            )
            .bind(profile_id)
            .bind(&record.ip)
            .bind(&record.country_code)
            .bind(&record.country_name)
            .bind(&record.region_name)
            .bind(&record.city)
            .bind(&record.geo_source)
            .bind(record.probe_updated_at)
            .bind(record.geo_updated_at)
            .bind(record.last_used_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_ip_records(&self, profile_id: &str) -> anyhow::Result<Vec<IpRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT ip, country_code, country_name, region_name, city, geo_source,
                   probe_updated_at, geo_updated_at, last_used_at
            FROM ip_records
            WHERE profile_id = ?1
            "#,
        )
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(IpRecord {
                    ip: row.try_get("ip")?,
                    country_code: row.try_get("country_code")?,
                    country_name: row.try_get("country_name")?,
                    region_name: row.try_get("region_name")?,
                    city: row.try_get("city")?,
                    geo_source: row.try_get("geo_source")?,
                    probe_updated_at: row.try_get("probe_updated_at")?,
                    geo_updated_at: row.try_get("geo_updated_at")?,
                    last_used_at: row.try_get("last_used_at")?,
                })
            })
            .collect()
    }

    async fn replace_probe_records(
        &self,
        profile_id: &str,
        records: &[ProbeRecord],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM probe_records WHERE profile_id = ?1")
            .bind(profile_id)
            .execute(&mut *tx)
            .await?;

        for record in records {
            sqlx::query(
                r#"
                INSERT INTO probe_records (
                  profile_id, proxy_name, ip, target_url, ok, latency_ms, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
            )
            .bind(profile_id)
            .bind(&record.proxy_name)
            .bind(&record.ip)
            .bind(&record.target_url)
            .bind(record.ok as i64)
            .bind(record.latency_ms.map(|x| x as i64))
            .bind(record.updated_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn upsert_probe_records(
        &self,
        profile_id: &str,
        records: &[ProbeRecord],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        for record in records {
            sqlx::query(
                r#"
                INSERT INTO probe_records (
                  profile_id, proxy_name, ip, target_url, ok, latency_ms, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(profile_id, proxy_name, ip, target_url) DO UPDATE SET
                  ok = excluded.ok,
                  latency_ms = excluded.latency_ms,
                  updated_at = excluded.updated_at
                "#,
            )
            .bind(profile_id)
            .bind(&record.proxy_name)
            .bind(&record.ip)
            .bind(&record.target_url)
            .bind(record.ok as i64)
            .bind(record.latency_ms.map(|x| x as i64))
            .bind(record.updated_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_probe_records(&self, profile_id: &str) -> anyhow::Result<Vec<ProbeRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT proxy_name, ip, target_url, ok, latency_ms, updated_at
            FROM probe_records
            WHERE profile_id = ?1
            "#,
        )
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let latency_ms: Option<i64> = row.try_get("latency_ms")?;
                Ok(ProbeRecord {
                    proxy_name: row.try_get("proxy_name")?,
                    ip: row.try_get("ip")?,
                    target_url: row.try_get("target_url")?,
                    ok: row.try_get::<i64, _>("ok")? != 0,
                    latency_ms: latency_ms.map(|x| x as u64),
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect()
    }

    async fn insert_session(
        &self,
        profile_id: &str,
        session: &SessionRecord,
    ) -> anyhow::Result<()> {
        self.insert_sessions(profile_id, std::slice::from_ref(session))
            .await
    }

    async fn insert_sessions(
        &self,
        profile_id: &str,
        sessions: &[SessionRecord],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        for session in sessions {
            sqlx::query(
                r#"
                INSERT INTO sessions (profile_id, session_id, listen, port, selected_ip, proxy_name, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(profile_id, session_id) DO UPDATE SET
                  listen = excluded.listen,
                  port = excluded.port,
                  selected_ip = excluded.selected_ip,
                  proxy_name = excluded.proxy_name,
                  created_at = excluded.created_at
                "#,
            )
            .bind(profile_id)
            .bind(&session.session_id)
            .bind(&session.listen)
            .bind(session.port as i64)
            .bind(&session.selected_ip)
            .bind(&session.proxy_name)
            .bind(session.created_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn insert_sessions_with_touch(
        &self,
        profile_id: &str,
        sessions: &[SessionRecord],
        last_used_at: i64,
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        for session in sessions {
            sqlx::query(
                r#"
                INSERT INTO sessions (profile_id, session_id, listen, port, selected_ip, proxy_name, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(profile_id, session_id) DO UPDATE SET
                  listen = excluded.listen,
                  port = excluded.port,
                  selected_ip = excluded.selected_ip,
                  proxy_name = excluded.proxy_name,
                  created_at = excluded.created_at
                "#,
            )
            .bind(profile_id)
            .bind(&session.session_id)
            .bind(&session.listen)
            .bind(session.port as i64)
            .bind(&session.selected_ip)
            .bind(&session.proxy_name)
            .bind(session.created_at)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                r#"
                INSERT INTO ip_records (
                  profile_id, ip, country_code, country_name, region_name, city,
                  geo_source, probe_updated_at, geo_updated_at, last_used_at
                )
                VALUES (?1, ?2, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?3)
                ON CONFLICT(profile_id, ip) DO UPDATE SET
                  last_used_at = excluded.last_used_at
                "#,
            )
            .bind(profile_id)
            .bind(&session.selected_ip)
            .bind(last_used_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn delete_session(&self, profile_id: &str, session_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM sessions WHERE profile_id = ?1 AND session_id = ?2")
            .bind(profile_id)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_sessions(&self, profile_id: &str) -> anyhow::Result<Vec<SessionRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT session_id, listen, port, selected_ip, proxy_name, created_at
            FROM sessions
            WHERE profile_id = ?1
            ORDER BY created_at ASC, session_id ASC
            "#,
        )
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let port: i64 = row.try_get("port")?;
                Ok(SessionRecord {
                    session_id: row.try_get("session_id")?,
                    listen: row.try_get("listen")?,
                    port: port as u16,
                    selected_ip: row.try_get("selected_ip")?,
                    proxy_name: row.try_get("proxy_name")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect()
    }

    async fn insert_api_key(&self, api_key: &ApiKeyRecord) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            INSERT INTO api_keys (
              key_id, name, secret_prefix, secret_salt, secret_hash,
              created_by_subject, scope_kind, created_at, last_used_at, revoked_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )
        .bind(&api_key.key_id)
        .bind(&api_key.name)
        .bind(&api_key.secret_prefix)
        .bind(&api_key.secret_salt)
        .bind(&api_key.secret_hash)
        .bind(&api_key.created_by_subject)
        .bind(api_key.profile_scope.kind.as_str())
        .bind(api_key.created_at)
        .bind(api_key.last_used_at)
        .bind(api_key.revoked_at)
        .execute(&mut *tx)
        .await?;

        for profile_id in &api_key.profile_scope.profile_ids {
            sqlx::query(
                r#"
                INSERT INTO api_key_profiles (key_id, profile_id)
                VALUES (?1, ?2)
                "#,
            )
            .bind(&api_key.key_id)
            .bind(profile_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn get_api_key(&self, key_id: &str) -> anyhow::Result<Option<ApiKeyRecord>> {
        let row = sqlx::query(
            r#"
            SELECT key_id, name, secret_prefix, secret_salt, secret_hash,
                   created_by_subject, scope_kind, created_at, last_used_at, revoked_at
            FROM api_keys
            WHERE key_id = ?1
            "#,
        )
        .bind(key_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let profile_ids = fetch_api_key_profile_ids(&self.pool, key_id).await?;
                Ok(Some(map_api_key_row(row, profile_ids)?))
            }
            None => Ok(None),
        }
    }

    async fn list_api_keys(&self, owner_subject: &str) -> anyhow::Result<Vec<ApiKeyRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT key_id, name, secret_prefix, secret_salt, secret_hash,
                   created_by_subject, scope_kind, created_at, last_used_at, revoked_at
            FROM api_keys
            WHERE created_by_subject = ?1
            ORDER BY created_at DESC, key_id ASC
            "#,
        )
        .bind(owner_subject)
        .fetch_all(&self.pool)
        .await?;

        let mut api_keys = Vec::with_capacity(rows.len());
        for row in rows {
            let key_id: String = row.try_get("key_id")?;
            let profile_ids = fetch_api_key_profile_ids(&self.pool, &key_id).await?;
            api_keys.push(map_api_key_row(row, profile_ids)?);
        }
        Ok(api_keys)
    }

    async fn revoke_api_key(
        &self,
        owner_subject: &str,
        key_id: &str,
        revoked_at: i64,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE api_keys
            SET revoked_at = ?3
            WHERE created_by_subject = ?1 AND key_id = ?2
            "#,
        )
        .bind(owner_subject)
        .bind(key_id)
        .bind(revoked_at)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn touch_api_key_last_used(&self, key_id: &str, last_used_at: i64) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE api_keys
            SET last_used_at = ?2
            WHERE key_id = ?1
            "#,
        )
        .bind(key_id)
        .bind(last_used_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn touch_ip_usage(
        &self,
        profile_id: &str,
        ip: &str,
        last_used_at: i64,
    ) -> anyhow::Result<()> {
        self.touch_ip_usages(profile_id, &[ip.to_string()], last_used_at)
            .await?;
        Ok(())
    }

    async fn touch_ip_usages(
        &self,
        profile_id: &str,
        ips: &[String],
        last_used_at: i64,
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        for ip in ips {
            sqlx::query(
                r#"
                INSERT INTO ip_records (
                  profile_id, ip, country_code, country_name, region_name, city,
                  geo_source, probe_updated_at, geo_updated_at, last_used_at
                )
                VALUES (?1, ?2, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?3)
                ON CONFLICT(profile_id, ip) DO UPDATE SET
                  last_used_at = excluded.last_used_at
                "#,
            )
            .bind(profile_id)
            .bind(ip)
            .bind(last_used_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn upsert_proxy_import_sync_config(
        &self,
        config: &ProxyImportSyncConfig,
    ) -> anyhow::Result<()> {
        let (source_type, source_value) = config.source.parts();
        sqlx::query(
            r#"
            INSERT INTO proxy_import_sync_configs (
              import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
              last_sync_due_at, last_sync_started_at, last_sync_finished_at,
              last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
              updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(import_id) DO UPDATE SET
              profile_id = excluded.profile_id,
              source_type = excluded.source_type,
              source_value = excluded.source_value,
              enabled = excluded.enabled,
              sync_every_sec = excluded.sync_every_sec,
              full_refresh_every_sec = excluded.full_refresh_every_sec,
              last_sync_due_at = excluded.last_sync_due_at,
              last_sync_started_at = excluded.last_sync_started_at,
              last_sync_finished_at = excluded.last_sync_finished_at,
              last_full_refresh_due_at = excluded.last_full_refresh_due_at,
              last_full_refresh_started_at = excluded.last_full_refresh_started_at,
              last_full_refresh_finished_at = excluded.last_full_refresh_finished_at,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(&config.import_id)
        .bind(&config.profile_id)
        .bind(source_type)
        .bind(source_value)
        .bind(config.enabled as i64)
        .bind(config.sync_every_sec as i64)
        .bind(config.full_refresh_every_sec as i64)
        .bind(config.last_sync_due_at)
        .bind(config.last_sync_started_at)
        .bind(config.last_sync_finished_at)
        .bind(config.last_full_refresh_due_at)
        .bind(config.last_full_refresh_started_at)
        .bind(config.last_full_refresh_finished_at)
        .bind(config.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_proxy_import_sync_config(
        &self,
        import_id: &str,
    ) -> anyhow::Result<Option<ProxyImportSyncConfig>> {
        let row = sqlx::query(
            r#"
            SELECT import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
                   last_sync_due_at, last_sync_started_at, last_sync_finished_at,
                   last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
                   updated_at
            FROM proxy_import_sync_configs
            WHERE import_id = ?1
            "#,
        )
        .bind(import_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_proxy_import_sync_config_row).transpose()
    }

    async fn list_proxy_import_sync_configs(&self) -> anyhow::Result<Vec<ProxyImportSyncConfig>> {
        let rows = sqlx::query(
            r#"
            SELECT import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
                   last_sync_due_at, last_sync_started_at, last_sync_finished_at,
                   last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
                   updated_at
            FROM proxy_import_sync_configs
            ORDER BY profile_id ASC, import_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(map_proxy_import_sync_config_row)
            .collect()
    }

    async fn list_proxy_import_sync_configs_for_profile(
        &self,
        profile_id: &str,
    ) -> anyhow::Result<Vec<ProxyImportSyncConfig>> {
        let rows = sqlx::query(
            r#"
            SELECT import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
                   last_sync_due_at, last_sync_started_at, last_sync_finished_at,
                   last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
                   updated_at
            FROM proxy_import_sync_configs
            WHERE profile_id = ?1
            ORDER BY import_id ASC
            "#,
        )
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(map_proxy_import_sync_config_row)
            .collect()
    }

    async fn delete_proxy_import_sync_config(&self, import_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM proxy_import_sync_configs WHERE import_id = ?1")
            .bind(import_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_profile_proxy_settings(
        &self,
        profile_id: &str,
    ) -> anyhow::Result<Option<ProfileProxySettings>> {
        let row = sqlx::query(
            r#"
            SELECT profile_id, use_global_proxies, updated_at
            FROM profile_proxy_settings
            WHERE profile_id = ?1
            "#,
        )
        .bind(profile_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_profile_proxy_settings_row).transpose()
    }

    async fn upsert_profile_proxy_settings(
        &self,
        settings: &ProfileProxySettings,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO profile_proxy_settings (profile_id, use_global_proxies, updated_at)
            VALUES (?1, ?2, strftime('%s','now'))
            ON CONFLICT(profile_id) DO UPDATE SET
              use_global_proxies = excluded.use_global_proxies,
              updated_at = strftime('%s','now')
            "#,
        )
        .bind(&settings.profile_id)
        .bind(settings.use_global_proxies as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn insert_task_run(&self, run: &TaskRunRecord) -> anyhow::Result<()> {
        persist_task_run(&self.pool, run).await
    }

    async fn update_task_run(&self, run: &TaskRunRecord) -> anyhow::Result<()> {
        persist_task_run(&self.pool, run).await
    }

    async fn get_task_run(&self, run_id: &str) -> anyhow::Result<Option<TaskRunRecord>> {
        let row = sqlx::query(
            r#"
            SELECT run_id, profile_id, kind, trigger, status, stage, progress_current, progress_total,
                   created_at, started_at, finished_at, summary_json, error_code, error_message, scope_json
            FROM task_runs
            WHERE run_id = ?1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_task_run_row).transpose()
    }

    async fn list_task_runs(&self, query: &TaskListQuery) -> anyhow::Result<Vec<TaskRunRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT run_id, profile_id, kind, trigger, status, stage, progress_current, progress_total,
                   created_at, started_at, finished_at, summary_json, error_code, error_message, scope_json
            FROM task_runs
            ORDER BY created_at DESC, run_id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut runs = rows
            .into_iter()
            .map(map_task_run_row)
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .filter(|run| matches_task_query(&run.as_summary(), query))
            .collect::<Vec<_>>();
        if let Some(cursor) = &query.cursor
            && let Some(position) = runs.iter().position(|run| &run.run_id == cursor)
        {
            runs = runs.into_iter().skip(position + 1).collect();
        }
        if let Some(limit) = query.limit {
            runs.truncate(limit);
        }
        Ok(runs)
    }

    async fn insert_task_run_event(&self, event: &TaskRunEventRecord) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO task_run_events (
              event_id, run_id, profile_id, at, level, stage, message, payload_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(&event.event_id)
        .bind(&event.run_id)
        .bind(&event.profile_id)
        .bind(event.at)
        .bind(event.level.as_str())
        .bind(event.stage.as_str())
        .bind(&event.message)
        .bind(
            event
                .payload_json
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_task_run_events(&self, run_id: &str) -> anyhow::Result<Vec<TaskRunEventRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, run_id, profile_id, at, level, stage, message, payload_json
            FROM task_run_events
            WHERE run_id = ?1
            ORDER BY at ASC, event_id ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_task_run_event_row).collect()
    }
}

async fn persist_proxy_inventory_node(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    node: &ProxyInventoryRecord,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO proxy_inventory_nodes (
          import_id, node_id, source_scope_type, source_scope_profile_id, source_type, source_value,
          allocation_scope_type, allocation_scope_profile_id,
          proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json,
          created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        ON CONFLICT(node_id) DO UPDATE SET
          import_id = excluded.import_id,
          source_scope_type = excluded.source_scope_type,
          source_scope_profile_id = excluded.source_scope_profile_id,
          source_type = excluded.source_type,
          source_value = excluded.source_value,
          allocation_scope_type = excluded.allocation_scope_type,
          allocation_scope_profile_id = excluded.allocation_scope_profile_id,
          proxy_name = excluded.proxy_name,
          proxy_type = excluded.proxy_type,
          server = excluded.server,
          resolved_ips_json = excluded.resolved_ips_json,
          raw_proxy_json = excluded.raw_proxy_json,
          created_at = excluded.created_at,
          updated_at = excluded.updated_at
        "#,
    )
    .bind(&node.import_id)
    .bind(&node.node_id)
    .bind(node.source_scope.kind())
    .bind(node.source_scope.profile_id())
    .bind("inventory")
    .bind(&node.import_id)
    .bind(node.allocation_scope.kind())
    .bind(node.allocation_scope.profile_id())
    .bind(&node.proxy_name)
    .bind(&node.proxy_type)
    .bind(&node.server)
    .bind(serde_json::to_string(&node.resolved_ips)?)
    .bind(serde_json::to_string(&node.raw_proxy)?)
    .bind(node.created_at)
    .bind(node.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn persist_proxy_import(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    import_record: &ProxyImportRecord,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO proxy_imports (
          import_id, name, import_kind, source_scope_type, source_scope_profile_id,
          source_type, source_value, allocation_scope_type, allocation_scope_profile_id,
          created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(import_id) DO UPDATE SET
          name = excluded.name,
          import_kind = excluded.import_kind,
          source_scope_type = excluded.source_scope_type,
          source_scope_profile_id = excluded.source_scope_profile_id,
          source_type = excluded.source_type,
          source_value = excluded.source_value,
          allocation_scope_type = excluded.allocation_scope_type,
          allocation_scope_profile_id = excluded.allocation_scope_profile_id,
          created_at = excluded.created_at,
          updated_at = excluded.updated_at
        "#,
    )
    .bind(&import_record.import_id)
    .bind(&import_record.name)
    .bind(match import_record.import_kind {
        crate::models::ProxyImportKind::Subscription => "subscription",
        crate::models::ProxyImportKind::SingleNode => "single_node",
    })
    .bind(import_record.source_scope.kind())
    .bind(import_record.source_scope.profile_id())
    .bind(&import_record.source_identity.source_type)
    .bind(&import_record.source_identity.source_value)
    .bind(import_record.allocation_scope.kind())
    .bind(import_record.allocation_scope.profile_id())
    .bind(import_record.created_at)
    .bind(import_record.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn map_proxy_inventory_row(row: sqlx::sqlite::SqliteRow) -> anyhow::Result<ProxyInventoryRecord> {
    let source_scope_type: String = row.try_get("source_scope_type")?;
    let allocation_scope_type: String = row.try_get("allocation_scope_type")?;
    let resolved_ips_json: String = row.try_get("resolved_ips_json")?;
    let raw_proxy_json: String = row.try_get("raw_proxy_json")?;
    Ok(ProxyInventoryRecord {
        import_id: row.try_get("import_id")?,
        node_id: row.try_get("node_id")?,
        source_scope: ProxyScope::from_parts(
            &source_scope_type,
            row.try_get("source_scope_profile_id")?,
        )
        .with_context(|| format!("unsupported proxy source scope type: {source_scope_type}"))?,
        allocation_scope: ProxyScope::from_parts(
            &allocation_scope_type,
            row.try_get("allocation_scope_profile_id")?,
        )
        .with_context(|| {
            format!("unsupported proxy allocation scope type: {allocation_scope_type}")
        })?,
        proxy_name: row.try_get("proxy_name")?,
        proxy_type: row.try_get("proxy_type")?,
        server: row.try_get("server")?,
        resolved_ips: serde_json::from_str(&resolved_ips_json)?,
        raw_proxy: serde_json::from_str(&raw_proxy_json)?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn map_proxy_import_row(row: sqlx::sqlite::SqliteRow) -> anyhow::Result<ProxyImportRecord> {
    let source_scope_type: String = row.try_get("source_scope_type")?;
    let allocation_scope_type: String = row.try_get("allocation_scope_type")?;
    let import_kind = match row.try_get::<String, _>("import_kind")?.as_str() {
        "subscription" => crate::models::ProxyImportKind::Subscription,
        "single_node" => crate::models::ProxyImportKind::SingleNode,
        other => return Err(anyhow!("unsupported proxy import kind: {other}")),
    };
    Ok(ProxyImportRecord {
        import_id: row.try_get("import_id")?,
        name: row.try_get("name")?,
        import_kind,
        source_scope: ProxyScope::from_parts(
            &source_scope_type,
            row.try_get("source_scope_profile_id")?,
        )
        .with_context(|| {
            format!("unsupported proxy import source scope type: {source_scope_type}")
        })?,
        source_identity: ProxyImportSourceIdentity {
            source_type: row.try_get("source_type")?,
            source_value: row.try_get("source_value")?,
        },
        allocation_scope: ProxyScope::from_parts(
            &allocation_scope_type,
            row.try_get("allocation_scope_profile_id")?,
        )
        .with_context(|| {
            format!("unsupported proxy import allocation scope type: {allocation_scope_type}")
        })?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn map_profile_proxy_settings_row(
    row: sqlx::sqlite::SqliteRow,
) -> anyhow::Result<ProfileProxySettings> {
    Ok(ProfileProxySettings {
        profile_id: row.try_get("profile_id")?,
        use_global_proxies: row.try_get::<i64, _>("use_global_proxies")? != 0,
    })
}

async fn fetch_api_key_profile_ids(pool: &SqlitePool, key_id: &str) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT profile_id
        FROM api_key_profiles
        WHERE key_id = ?1
        ORDER BY profile_id ASC
        "#,
    )
    .bind(key_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| row.try_get("profile_id").map_err(anyhow::Error::from))
        .collect()
}

fn map_api_key_row(
    row: sqlx::sqlite::SqliteRow,
    profile_ids: Vec<String>,
) -> anyhow::Result<ApiKeyRecord> {
    let scope_kind: String = row.try_get("scope_kind")?;
    Ok(ApiKeyRecord {
        key_id: row.try_get("key_id")?,
        name: row.try_get("name")?,
        secret_prefix: row.try_get("secret_prefix")?,
        secret_salt: row.try_get("secret_salt")?,
        secret_hash: row.try_get("secret_hash")?,
        created_by_subject: row.try_get("created_by_subject")?,
        profile_scope: ApiKeyProfileScope {
            kind: ApiKeyProfileScopeKind::parse(&scope_kind)
                .with_context(|| format!("unsupported api key scope kind: {scope_kind}"))?,
            profile_ids,
        },
        created_at: row.try_get("created_at")?,
        last_used_at: row.try_get("last_used_at")?,
        revoked_at: row.try_get("revoked_at")?,
    })
}

fn map_proxy_import_sync_config_row(
    row: sqlx::sqlite::SqliteRow,
) -> anyhow::Result<ProxyImportSyncConfig> {
    let source_type: String = row.try_get("source_type")?;
    let source_value: String = row.try_get("source_value")?;
    let source = SubscriptionSource::from_parts(&source_type, source_value)
        .with_context(|| format!("unsupported profile sync source type: {source_type}"))?;
    let sync_every_sec: i64 = row.try_get("sync_every_sec")?;
    let full_refresh_every_sec: i64 = row.try_get("full_refresh_every_sec")?;
    Ok(ProxyImportSyncConfig {
        import_id: row.try_get("import_id")?,
        profile_id: row.try_get("profile_id")?,
        source,
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        sync_every_sec: sync_every_sec as u64,
        full_refresh_every_sec: full_refresh_every_sec as u64,
        last_sync_due_at: row.try_get("last_sync_due_at")?,
        last_sync_started_at: row.try_get("last_sync_started_at")?,
        last_sync_finished_at: row.try_get("last_sync_finished_at")?,
        last_full_refresh_due_at: row.try_get("last_full_refresh_due_at")?,
        last_full_refresh_started_at: row.try_get("last_full_refresh_started_at")?,
        last_full_refresh_finished_at: row.try_get("last_full_refresh_finished_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn merge_proxy_import_records(
    existing: &ProxyImportRecord,
    incoming: &ProxyImportRecord,
    import_id: &str,
) -> ProxyImportRecord {
    let (preferred, fallback) = if incoming.updated_at >= existing.updated_at {
        (incoming, existing)
    } else {
        (existing, incoming)
    };

    ProxyImportRecord {
        import_id: import_id.to_string(),
        name: preferred.name.clone().or_else(|| fallback.name.clone()),
        import_kind: preferred.import_kind,
        source_scope: preferred.source_scope.clone(),
        source_identity: preferred.source_identity.clone(),
        allocation_scope: preferred.allocation_scope.clone(),
        created_at: existing.created_at.min(incoming.created_at),
        updated_at: existing.updated_at.max(incoming.updated_at),
    }
}

fn merge_proxy_inventory_records(
    existing: &ProxyInventoryRecord,
    incoming: &ProxyInventoryRecord,
    node_id: &str,
) -> ProxyInventoryRecord {
    let (preferred, fallback) = if incoming.updated_at >= existing.updated_at {
        (incoming, existing)
    } else {
        (existing, incoming)
    };
    let mut resolved_ips = preferred.resolved_ips.clone();
    for ip in &fallback.resolved_ips {
        if !resolved_ips.contains(ip) {
            resolved_ips.push(ip.clone());
        }
    }

    ProxyInventoryRecord {
        import_id: preferred.import_id.clone(),
        node_id: node_id.to_string(),
        source_scope: preferred.source_scope.clone(),
        allocation_scope: preferred.allocation_scope.clone(),
        proxy_name: preferred.proxy_name.clone(),
        proxy_type: preferred.proxy_type.clone(),
        server: preferred.server.clone(),
        resolved_ips,
        raw_proxy: preferred.raw_proxy.clone(),
        created_at: existing.created_at.min(incoming.created_at),
        updated_at: existing.updated_at.max(incoming.updated_at),
    }
}

fn merge_proxy_import_sync_configs(
    existing: &ProxyImportSyncConfig,
    incoming: &ProxyImportSyncConfig,
    import_id: &str,
) -> ProxyImportSyncConfig {
    let (preferred, fallback) = if incoming.updated_at >= existing.updated_at {
        (incoming, existing)
    } else {
        (existing, incoming)
    };

    ProxyImportSyncConfig {
        import_id: import_id.to_string(),
        profile_id: preferred.profile_id.clone(),
        source: preferred.source.clone(),
        enabled: preferred.enabled,
        sync_every_sec: preferred.sync_every_sec,
        full_refresh_every_sec: preferred.full_refresh_every_sec,
        last_sync_due_at: preferred.last_sync_due_at.or(fallback.last_sync_due_at),
        last_sync_started_at: preferred
            .last_sync_started_at
            .or(fallback.last_sync_started_at),
        last_sync_finished_at: preferred
            .last_sync_finished_at
            .or(fallback.last_sync_finished_at),
        last_full_refresh_due_at: preferred
            .last_full_refresh_due_at
            .or(fallback.last_full_refresh_due_at),
        last_full_refresh_started_at: preferred
            .last_full_refresh_started_at
            .or(fallback.last_full_refresh_started_at),
        last_full_refresh_finished_at: preferred
            .last_full_refresh_finished_at
            .or(fallback.last_full_refresh_finished_at),
        updated_at: preferred.updated_at.max(fallback.updated_at),
    }
}

fn stable_proxy_import_id(
    source_scope: &ProxyScope,
    source_identity: &ProxyImportSourceIdentity,
) -> String {
    ids::stable_import_id(&source_scope.key(), &source_identity.key())
}

fn reserve_unique_id<F>(reserved: &mut HashSet<String>, mut generate: F) -> String
where
    F: FnMut() -> String,
{
    loop {
        let candidate = generate();
        if !reserved.contains(&candidate) {
            return candidate;
        }
    }
}

async fn persist_task_run(pool: &SqlitePool, run: &TaskRunRecord) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO task_runs (
          run_id, profile_id, kind, trigger, status, stage, progress_current, progress_total,
          created_at, started_at, finished_at, summary_json, error_code, error_message, scope_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        ON CONFLICT(run_id) DO UPDATE SET
          profile_id = excluded.profile_id,
          kind = excluded.kind,
          trigger = excluded.trigger,
          status = excluded.status,
          stage = excluded.stage,
          progress_current = excluded.progress_current,
          progress_total = excluded.progress_total,
          created_at = excluded.created_at,
          started_at = excluded.started_at,
          finished_at = excluded.finished_at,
          summary_json = excluded.summary_json,
          error_code = excluded.error_code,
          error_message = excluded.error_message,
          scope_json = excluded.scope_json
        "#,
    )
    .bind(&run.run_id)
    .bind(&run.profile_id)
    .bind(run.kind.as_str())
    .bind(run.trigger.as_str())
    .bind(run.status.as_str())
    .bind(run.stage.as_str())
    .bind(run.progress_current.map(|value| value as i64))
    .bind(run.progress_total.map(|value| value as i64))
    .bind(run.created_at)
    .bind(run.started_at)
    .bind(run.finished_at)
    .bind(
        run.summary_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?,
    )
    .bind(&run.error_code)
    .bind(&run.error_message)
    .bind(Some(serde_json::to_string(&run.scope)?))
    .execute(pool)
    .await?;
    Ok(())
}

fn map_task_run_row(row: sqlx::sqlite::SqliteRow) -> anyhow::Result<TaskRunRecord> {
    let kind: String = row.try_get("kind")?;
    let trigger: String = row.try_get("trigger")?;
    let status: String = row.try_get("status")?;
    let stage: String = row.try_get("stage")?;
    let summary_json: Option<String> = row.try_get("summary_json")?;
    let scope_json: Option<String> = row.try_get("scope_json")?;
    let progress_current: Option<i64> = row.try_get("progress_current")?;
    let progress_total: Option<i64> = row.try_get("progress_total")?;
    Ok(TaskRunRecord {
        run_id: row.try_get("run_id")?,
        profile_id: row.try_get("profile_id")?,
        kind: TaskRunKind::parse(&kind)
            .with_context(|| format!("unsupported task kind: {kind}"))?,
        trigger: TaskRunTrigger::parse(&trigger)
            .with_context(|| format!("unsupported task trigger: {trigger}"))?,
        status: TaskRunStatus::parse(&status)
            .with_context(|| format!("unsupported task status: {status}"))?,
        stage: TaskRunStage::parse(&stage)
            .with_context(|| format!("unsupported task stage: {stage}"))?,
        progress_current: progress_current.map(|value| value as u64),
        progress_total: progress_total.map(|value| value as u64),
        created_at: row.try_get("created_at")?,
        started_at: row.try_get("started_at")?,
        finished_at: row.try_get("finished_at")?,
        summary_json: summary_json
            .map(|value| serde_json::from_str(&value))
            .transpose()?,
        error_code: row.try_get("error_code")?,
        error_message: row.try_get("error_message")?,
        scope: scope_json
            .map(|value| serde_json::from_str(&value))
            .transpose()?
            .unwrap_or_default(),
    })
}

fn map_task_run_event_row(row: sqlx::sqlite::SqliteRow) -> anyhow::Result<TaskRunEventRecord> {
    let level: String = row.try_get("level")?;
    let stage: String = row.try_get("stage")?;
    let payload_json: Option<String> = row.try_get("payload_json")?;
    Ok(TaskRunEventRecord {
        event_id: row.try_get("event_id")?,
        run_id: row.try_get("run_id")?,
        profile_id: row.try_get("profile_id")?,
        at: row.try_get("at")?,
        level: TaskEventLevel::parse(&level)
            .with_context(|| format!("unsupported task event level: {level}"))?,
        stage: TaskRunStage::parse(&stage)
            .with_context(|| format!("unsupported task event stage: {stage}"))?,
        message: row.try_get("message")?,
        payload_json: payload_json
            .map(|value| serde_json::from_str(&value))
            .transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{SqliteStore, stable_proxy_import_id};
    use crate::{
        auth::issue_api_key,
        ids,
        models::{
            ApiKeyProfileScope, ProxyImportSourceIdentity, ProxyNode, ProxyScope,
            SubscriptionSource, TaskListQuery,
        },
        store::BrokerStore,
    };
    use sqlx::{Executor, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions};

    fn temp_store_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "proxy-broker-store-{}.db",
            ids::random_temp_suffix()
        ))
    }

    async fn open_temp_store() -> (SqliteStore, std::path::PathBuf) {
        let path = temp_store_path();
        let store = SqliteStore::open(&path)
            .await
            .expect("sqlite store should open");
        (store, path)
    }

    async fn seed_legacy_proxy_inventory_store(
        path: &Path,
        legacy_configs: &[(&str, &str)],
        inventory_rows: &[(&str, &str, &str, &str, i64, i64)],
    ) {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .expect("sqlite parent should exist");
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(path)
                    .create_if_missing(true),
            )
            .await
            .expect("legacy sqlite should open");

        sqlx::query(
            r#"
            CREATE TABLE proxy_inventory_nodes (
              node_id TEXT PRIMARY KEY,
              source_scope_type TEXT NOT NULL,
              source_scope_profile_id TEXT,
              allocation_scope_type TEXT NOT NULL,
              allocation_scope_profile_id TEXT,
              proxy_name TEXT NOT NULL,
              proxy_type TEXT NOT NULL,
              server TEXT NOT NULL,
              resolved_ips_json TEXT NOT NULL,
              raw_proxy_json TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("legacy proxy_inventory_nodes schema should be created");

        sqlx::query(
            r#"
            CREATE TABLE profile_sync_configs (
              profile_id TEXT PRIMARY KEY,
              source_type TEXT NOT NULL,
              source_value TEXT NOT NULL,
              enabled INTEGER NOT NULL,
              sync_every_sec INTEGER NOT NULL,
              full_refresh_every_sec INTEGER NOT NULL,
              last_sync_due_at INTEGER,
              last_sync_started_at INTEGER,
              last_sync_finished_at INTEGER,
              last_full_refresh_due_at INTEGER,
              last_full_refresh_started_at INTEGER,
              last_full_refresh_finished_at INTEGER,
              updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("legacy profile_sync_configs schema should be created");

        for &(profile_id, source_value) in legacy_configs {
            sqlx::query(
                r#"
                INSERT INTO profile_sync_configs (
                  profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
            )
            .bind(profile_id)
            .bind("url")
            .bind(source_value)
            .bind(1_i64)
            .bind(300_i64)
            .bind(3600_i64)
            .bind(20_i64)
            .execute(&pool)
            .await
            .expect("legacy profile sync config should be seeded");
        }

        for &(node_id, profile_id, proxy_name, server, created_at, updated_at) in inventory_rows {
            sqlx::query(
                r#"
                INSERT INTO proxy_inventory_nodes (
                  node_id, source_scope_type, source_scope_profile_id, allocation_scope_type, allocation_scope_profile_id,
                  proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                "#,
            )
            .bind(node_id)
            .bind("profile")
            .bind(profile_id)
            .bind("profile")
            .bind(profile_id)
            .bind(proxy_name)
            .bind("socks5")
            .bind(server)
            .bind(serde_json::json!([server]).to_string())
            .bind(
                serde_json::json!({
                    "name": proxy_name,
                    "type": "socks5",
                    "server": server
                })
                .to_string(),
            )
            .bind(created_at)
            .bind(updated_at)
            .execute(&pool)
            .await
            .expect("legacy proxy inventory row should be seeded");
        }

        pool.close().await;
    }

    async fn seed_current_schema_legacy_ids(path: &Path) {
        let bootstrap = SqliteStore::open(path)
            .await
            .expect("current sqlite schema should bootstrap");
        drop(bootstrap);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(path)
                    .create_if_missing(true),
            )
            .await
            .expect("sqlite should open for legacy-id seeding");

        sqlx::query("INSERT INTO profiles (profile_id, created_at) VALUES (?1, ?2)")
            .bind("legacy-profile")
            .bind(1_i64)
            .execute(&pool)
            .await
            .expect("profile row should seed");

        sqlx::query(
            r#"
            INSERT INTO sessions (profile_id, session_id, listen, port, selected_ip, proxy_name, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind("legacy-profile")
        .bind("123e4567-e89b-12d3-a456-426614174000")
        .bind("127.0.0.1")
        .bind(12080_i64)
        .bind("1.1.1.1")
        .bind("node-a")
        .bind(10_i64)
        .execute(&pool)
        .await
        .expect("legacy session should seed");

        sqlx::query(
            r#"
            INSERT INTO task_runs (
              run_id, profile_id, kind, trigger, status, stage, progress_current, progress_total,
              created_at, started_at, finished_at, summary_json, error_code, error_message, scope_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, ?7, NULL, NULL, NULL, NULL, NULL, ?8)
            "#,
        )
        .bind("223e4567-e89b-12d3-a456-426614174000")
        .bind("legacy-profile")
        .bind("subscription_sync")
        .bind("schedule")
        .bind("queued")
        .bind("queued")
        .bind(11_i64)
        .bind(serde_json::json!({ "type": "all" }).to_string())
        .execute(&pool)
        .await
        .expect("legacy task run should seed");

        sqlx::query(
            r#"
            INSERT INTO task_run_events (event_id, run_id, profile_id, at, level, stage, message, payload_json)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)
            "#,
        )
        .bind("323e4567-e89b-12d3-a456-426614174000")
        .bind("223e4567-e89b-12d3-a456-426614174000")
        .bind("legacy-profile")
        .bind(12_i64)
        .bind("info")
        .bind("queued")
        .bind("queued")
        .execute(&pool)
        .await
        .expect("legacy task event should seed");

        let old_manual_import_id = "423e4567-e89b-12d3-a456-426614174000";
        sqlx::query(
            r#"
            INSERT INTO proxy_imports (
              import_id, name, import_kind, source_scope_type, source_scope_profile_id,
              source_type, source_value, allocation_scope_type, allocation_scope_profile_id,
              created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
        )
        .bind(old_manual_import_id)
        .bind("manual group")
        .bind("single_node")
        .bind("profile")
        .bind("legacy-profile")
        .bind("manual")
        .bind(old_manual_import_id)
        .bind("profile")
        .bind("legacy-profile")
        .bind(13_i64)
        .bind(14_i64)
        .execute(&pool)
        .await
        .expect("legacy manual import should seed");

        sqlx::query(
            r#"
            INSERT INTO proxy_inventory_nodes (
              node_id, import_id, source_scope_type, source_scope_profile_id, source_type, source_value,
              allocation_scope_type, allocation_scope_profile_id,
              proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
        )
        .bind("523e4567-e89b-12d3-a456-426614174000")
        .bind(old_manual_import_id)
        .bind("profile")
        .bind("legacy-profile")
        .bind("manual")
        .bind(old_manual_import_id)
        .bind("profile")
        .bind("legacy-profile")
        .bind("manual-node")
        .bind("socks5")
        .bind("3.3.3.3")
        .bind(serde_json::json!(["3.3.3.3"]).to_string())
        .bind(
            serde_json::json!({
                "name": "manual-node",
                "type": "socks5",
                "server": "3.3.3.3"
            })
            .to_string(),
        )
        .bind(13_i64)
        .bind(14_i64)
        .execute(&pool)
        .await
        .expect("legacy manual inventory row should seed");

        sqlx::query(
            r#"
            INSERT INTO proxy_import_sync_configs (
              import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
              last_sync_due_at, last_sync_started_at, last_sync_finished_at,
              last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, 1, 300, 3600, NULL, NULL, NULL, NULL, NULL, NULL, ?5)
            "#,
        )
        .bind("623e4567-e89b-12d3-a456-426614174000")
        .bind("legacy-profile")
        .bind("url")
        .bind("https://example.com/sync.yaml")
        .bind(15_i64)
        .execute(&pool)
        .await
        .expect("legacy sync config should seed");

        sqlx::query(
            r#"
            INSERT INTO api_keys (
              key_id, name, secret_prefix, secret_salt, secret_hash,
              created_by_subject, scope_kind, created_at, last_used_at, revoked_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL)
            "#,
        )
        .bind("723e4567e89b12d3a456426614174000")
        .bind("old key")
        .bind("pbk_old")
        .bind("823e4567e89b12d3a456426614174000")
        .bind("hash")
        .bind("admin@example.com")
        .bind("selected_profiles")
        .bind(16_i64)
        .execute(&pool)
        .await
        .expect("legacy api key should seed");
        sqlx::query("INSERT INTO api_key_profiles (key_id, profile_id) VALUES (?1, ?2)")
            .bind("723e4567e89b12d3a456426614174000")
            .bind("legacy-profile")
            .execute(&pool)
            .await
            .expect("legacy api key profile scope should seed");

        pool.close().await;
    }

    fn sample_node(profile_name: &str, ip: &str) -> ProxyNode {
        ProxyNode {
            proxy_name: profile_name.to_string(),
            proxy_type: "socks5".to_string(),
            server: ip.to_string(),
            resolved_ips: vec![ip.to_string()],
            raw_proxy: serde_json::json!({
                "name": profile_name,
                "type": "socks5",
                "server": ip
            }),
        }
    }

    #[tokio::test]
    async fn create_profile_lists_empty_profile_without_other_records() {
        let (store, path) = open_temp_store().await;

        store
            .create_profile("empty-profile", 1)
            .await
            .expect("create should succeed");

        let profiles = store.list_profiles().await.expect("list should succeed");
        assert_eq!(profiles, vec!["empty-profile"]);

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn list_profiles_keeps_legacy_profiles_from_runtime_tables() {
        let (store, path) = open_temp_store().await;

        store
            .replace_subscription("legacy-profile", &[sample_node("node-a", "1.1.1.1")])
            .await
            .expect("seed subscription should succeed");

        let profiles = store.list_profiles().await.expect("list should succeed");
        assert_eq!(profiles, vec!["legacy-profile"]);

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn api_keys_round_trip_and_touch_last_used() {
        let (store, path) = open_temp_store().await;
        let issued = issue_api_key(
            "ci-bot",
            "admin@example.com",
            ApiKeyProfileScope::selected(["alpha".to_string()]),
        );

        store
            .insert_api_key(&issued.record)
            .await
            .expect("insert should succeed");
        store
            .touch_api_key_last_used(&issued.record.key_id, 77)
            .await
            .expect("touch should succeed");

        let fetched = store
            .get_api_key(&issued.record.key_id)
            .await
            .expect("get should succeed")
            .expect("api key should exist");
        assert_eq!(fetched.last_used_at, Some(77));

        let listed = store
            .list_api_keys("admin@example.com")
            .await
            .expect("list should succeed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "ci-bot");

        let other_owner_keys = store
            .list_api_keys("viewer@example.com")
            .await
            .expect("list should succeed");
        assert!(other_owner_keys.is_empty());

        let other_owner_revoked = store
            .revoke_api_key("viewer@example.com", &issued.record.key_id, 66)
            .await
            .expect("revoke should succeed");
        assert!(!other_owner_revoked);

        let revoked = store
            .revoke_api_key("admin@example.com", &issued.record.key_id, 99)
            .await
            .expect("revoke should succeed");
        assert!(revoked);

        let revoked_record = store
            .get_api_key(&issued.record.key_id)
            .await
            .expect("get should succeed")
            .expect("api key should exist");
        assert_eq!(revoked_record.revoked_at, Some(99));

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn open_migrates_legacy_proxy_inventory_before_creating_import_indexes() {
        let path = temp_store_path();
        seed_legacy_proxy_inventory_store(
            &path,
            &[("legacy-profile", "https://example.com/legacy.yaml")],
            &[("legacy-node", "legacy-profile", "node-a", "1.1.1.1", 10, 20)],
        )
        .await;

        let store = SqliteStore::open(&path)
            .await
            .expect("legacy sqlite store should migrate successfully");

        assert!(
            store
                .table_has_column("proxy_inventory_nodes", "import_id")
                .await
                .expect("import_id lookup should succeed"),
            "legacy proxy_inventory_nodes should gain import_id before index creation"
        );
        assert!(
            store
                .table_has_column("proxy_inventory_nodes", "source_type")
                .await
                .expect("source_type lookup should succeed"),
            "legacy proxy_inventory_nodes should gain source_type during migration"
        );
        assert!(
            store
                .table_has_column("proxy_inventory_nodes", "source_value")
                .await
                .expect("source_value lookup should succeed"),
            "legacy proxy_inventory_nodes should gain source_value during migration"
        );

        let expected_scope = ProxyScope::profile("legacy-profile");
        let expected_source = ProxyImportSourceIdentity::from_source(&SubscriptionSource::Url(
            "https://example.com/legacy.yaml".to_string(),
        ));
        let expected_import_id = stable_proxy_import_id(&expected_scope, &expected_source);

        let imports = store
            .list_proxy_imports()
            .await
            .expect("proxy imports should be listed after migration");
        assert_eq!(
            imports.len(),
            1,
            "legacy inventory should backfill one import"
        );
        assert_eq!(imports[0].import_id, expected_import_id);
        assert_eq!(imports[0].source_scope, expected_scope);
        assert_eq!(
            imports[0].source_identity.source_type,
            expected_source.source_type
        );
        assert_eq!(
            imports[0].source_identity.source_value,
            expected_source.source_value
        );

        let nodes = store
            .list_proxy_inventory_for_import(&expected_import_id)
            .await
            .expect("inventory nodes should remain accessible after migration");
        assert_eq!(nodes.len(), 1);
        assert_eq!(
            nodes[0].node_id,
            ids::stable_proxy_inventory_node_id(&expected_import_id, "node-a")
        );
        assert_eq!(nodes[0].import_id, expected_import_id);
        assert_eq!(nodes[0].server, "1.1.1.1");

        let sync_config = store
            .get_proxy_import_sync_config(&expected_import_id)
            .await
            .expect("proxy import sync config should load")
            .expect("legacy profile sync config should be backfilled");
        assert_eq!(sync_config.profile_id, "legacy-profile");
        match sync_config.source {
            SubscriptionSource::Url(value) => {
                assert_eq!(value, "https://example.com/legacy.yaml")
            }
            other => panic!("unexpected backfilled source: {other:?}"),
        }

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn open_backfills_legacy_import_ids_before_unique_proxy_name_index() {
        let path = temp_store_path();
        seed_legacy_proxy_inventory_store(
            &path,
            &[
                ("legacy-a", "https://example.com/a.yaml"),
                ("legacy-b", "https://example.com/b.yaml"),
            ],
            &[
                ("legacy-node-a", "legacy-a", "same-name", "1.1.1.1", 10, 20),
                ("legacy-node-b", "legacy-b", "same-name", "2.2.2.2", 11, 21),
            ],
        )
        .await;

        let store = SqliteStore::open(&path)
            .await
            .expect("legacy sqlite store should migrate duplicate proxy names successfully");

        let expected_import_ids = [
            (
                ProxyScope::profile("legacy-a"),
                ProxyImportSourceIdentity::from_source(&SubscriptionSource::Url(
                    "https://example.com/a.yaml".to_string(),
                )),
            ),
            (
                ProxyScope::profile("legacy-b"),
                ProxyImportSourceIdentity::from_source(&SubscriptionSource::Url(
                    "https://example.com/b.yaml".to_string(),
                )),
            ),
        ]
        .into_iter()
        .map(|(scope, source)| stable_proxy_import_id(&scope, &source))
        .collect::<std::collections::BTreeSet<_>>();

        let imports = store
            .list_proxy_imports()
            .await
            .expect("proxy imports should be listed after duplicate-name migration");
        let actual_import_ids = imports
            .iter()
            .map(|record| record.import_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual_import_ids, expected_import_ids);

        for import_id in actual_import_ids {
            let nodes = store
                .list_proxy_inventory_for_import(&import_id)
                .await
                .expect("inventory rows should stay readable after duplicate-name migration");
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].proxy_name, "same-name");
        }

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn open_clears_legacy_single_profile_api_keys() {
        let path = temp_store_path();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true),
            )
            .await
            .expect("legacy sqlite pool should open");

        pool.execute(
            r#"
            CREATE TABLE api_keys (
              key_id TEXT PRIMARY KEY,
              profile_id TEXT NOT NULL,
              name TEXT NOT NULL,
              secret_prefix TEXT NOT NULL,
              secret_salt TEXT NOT NULL,
              secret_hash TEXT NOT NULL,
              created_by_subject TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              last_used_at INTEGER,
              revoked_at INTEGER
            )
            "#,
        )
        .await
        .expect("legacy api_keys table should create");
        pool.execute("CREATE UNIQUE INDEX idx_api_keys_secret_hash ON api_keys(secret_hash)")
            .await
            .expect("legacy api key hash index should create");
        pool.execute(
            r#"
            INSERT INTO api_keys (
              key_id, profile_id, name, secret_prefix, secret_salt, secret_hash, created_by_subject, created_at
            ) VALUES (
              'key_legacy', 'legacy-profile', 'legacy-bot', 'pbk_key_legacy_prefix', 'salt', 'hash', 'admin@example.com', 123
            )
            "#,
        )
        .await
        .expect("legacy api key should insert");
        pool.close().await;

        let store = SqliteStore::open(&path)
            .await
            .expect("sqlite store should migrate legacy schema");
        let migrated = store
            .get_api_key("key_legacy")
            .await
            .expect("lookup should succeed");
        assert!(migrated.is_none(), "legacy api key should be invalidated");
        let listed = store
            .list_api_keys("admin@example.com")
            .await
            .expect("api key list should load");
        assert!(
            listed.is_empty(),
            "legacy api key should not remain visible"
        );
        let profiles = store.list_profiles().await.expect("list should succeed");
        assert!(
            profiles.is_empty(),
            "legacy api key migration should not leave orphaned profile scope rows"
        );

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn open_rewrites_legacy_random_ids_and_clears_legacy_api_keys() {
        let path = temp_store_path();
        seed_current_schema_legacy_ids(&path).await;

        let store = SqliteStore::open(&path)
            .await
            .expect("legacy-id rows should migrate successfully");

        let sessions = store
            .list_sessions("legacy-profile")
            .await
            .expect("sessions should load");
        assert_eq!(sessions.len(), 1);
        assert!(ids::is_prefixed_short_id(
            &sessions[0].session_id,
            "sess",
            ids::ENTITY_ID_BODY_LEN
        ));

        let runs = store
            .list_task_runs(&TaskListQuery::default())
            .await
            .expect("task runs should load");
        assert_eq!(runs.len(), 1);
        assert!(ids::is_prefixed_short_id(
            &runs[0].run_id,
            "run",
            ids::ENTITY_ID_BODY_LEN
        ));
        let events = store
            .list_task_run_events(&runs[0].run_id)
            .await
            .expect("task events should load");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].run_id, runs[0].run_id);
        assert!(ids::is_prefixed_short_id(
            &events[0].event_id,
            "evt",
            ids::ENTITY_ID_BODY_LEN
        ));

        let imports = store
            .list_proxy_imports()
            .await
            .expect("proxy imports should load");
        assert_eq!(imports.len(), 1);
        let migrated_import = &imports[0];
        assert!(ids::is_prefixed_short_id(
            &migrated_import.import_id,
            "imp",
            ids::ENTITY_ID_BODY_LEN
        ));
        assert_eq!(migrated_import.source_identity.source_type, "manual");
        assert_eq!(
            migrated_import.source_identity.source_value,
            migrated_import.import_id
        );

        let nodes = store
            .list_proxy_inventory_for_import(&migrated_import.import_id)
            .await
            .expect("migrated nodes should load");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].import_id, migrated_import.import_id);
        assert_eq!(
            nodes[0].node_id,
            ids::stable_proxy_inventory_node_id(&migrated_import.import_id, "manual-node")
        );

        let expected_sync_import_id = ids::stable_import_id(
            &ProxyScope::profile("legacy-profile").key(),
            &ProxyImportSourceIdentity::from_source(&SubscriptionSource::Url(
                "https://example.com/sync.yaml".to_string(),
            ))
            .key(),
        );
        let sync_config = store
            .get_proxy_import_sync_config(&expected_sync_import_id)
            .await
            .expect("sync config lookup should succeed")
            .expect("stable sync config should migrate to new import id");
        match sync_config.source {
            SubscriptionSource::Url(value) => assert_eq!(value, "https://example.com/sync.yaml"),
            other => panic!("unexpected sync source after migration: {other:?}"),
        }

        let api_keys = store
            .list_api_keys("admin@example.com")
            .await
            .expect("api key list should load");
        assert!(api_keys.is_empty(), "legacy api keys should be cleared");

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn open_merges_legacy_proxy_import_sync_configs_on_short_id_conflict() {
        let path = temp_store_path();
        let bootstrap = SqliteStore::open(&path)
            .await
            .expect("current sqlite schema should bootstrap");
        drop(bootstrap);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true),
            )
            .await
            .expect("sqlite should open for sync-config conflict seed");

        sqlx::query("INSERT INTO profiles (profile_id, created_at) VALUES (?1, ?2)")
            .bind("legacy-profile")
            .bind(1_i64)
            .execute(&pool)
            .await
            .expect("profile row should seed");

        sqlx::query(
            r#"
            INSERT INTO profile_sync_configs (
              profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
              last_sync_due_at, last_sync_started_at, last_sync_finished_at,
              last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
              updated_at
            )
            VALUES (?1, ?2, ?3, 1, 300, 3600, 50, NULL, NULL, 60, NULL, NULL, 70)
            "#,
        )
        .bind("legacy-profile")
        .bind("url")
        .bind("https://example.com/sync.yaml")
        .execute(&pool)
        .await
        .expect("legacy profile sync config should seed");

        sqlx::query(
            r#"
            INSERT INTO proxy_import_sync_configs (
              import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
              last_sync_due_at, last_sync_started_at, last_sync_finished_at,
              last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
              updated_at
            )
            VALUES (?1, ?2, ?3, ?4, 0, 600, 7200, 80, 81, 82, 90, 91, 92, 100)
            "#,
        )
        .bind("723e4567-e89b-12d3-a456-426614174000")
        .bind("legacy-profile")
        .bind("url")
        .bind("https://example.com/sync.yaml")
        .execute(&pool)
        .await
        .expect("legacy import-keyed sync config should seed");

        pool.close().await;

        let store = SqliteStore::open(&path)
            .await
            .expect("sqlite store should merge duplicate sync configs during short-id migration");

        let expected_import_id = ids::stable_import_id(
            &ProxyScope::profile("legacy-profile").key(),
            &ProxyImportSourceIdentity::from_source(&SubscriptionSource::Url(
                "https://example.com/sync.yaml".to_string(),
            ))
            .key(),
        );
        let configs = store
            .list_proxy_import_sync_configs_for_profile("legacy-profile")
            .await
            .expect("sync configs should list");
        assert_eq!(
            configs.len(),
            1,
            "migration should collapse legacy and stable sync configs into one row"
        );
        assert_eq!(configs[0].import_id, expected_import_id);
        assert!(!configs[0].enabled);
        assert_eq!(configs[0].sync_every_sec, 600);
        assert_eq!(configs[0].full_refresh_every_sec, 7200);
        assert_eq!(configs[0].last_sync_due_at, Some(80));
        assert_eq!(configs[0].last_sync_started_at, Some(81));
        assert_eq!(configs[0].last_sync_finished_at, Some(82));
        assert_eq!(configs[0].last_full_refresh_due_at, Some(90));
        assert_eq!(configs[0].last_full_refresh_started_at, Some(91));
        assert_eq!(configs[0].last_full_refresh_finished_at, Some(92));
        assert_eq!(configs[0].updated_at, 100);
        assert!(
            store
                .get_proxy_import_sync_config("723e4567-e89b-12d3-a456-426614174000")
                .await
                .expect("legacy sync config lookup should succeed")
                .is_none(),
            "legacy import-keyed sync config should be removed after merge"
        );

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn open_keeps_sync_config_source_when_legacy_inventory_import_rewrites_import_id() {
        let path = temp_store_path();
        let bootstrap = SqliteStore::open(&path)
            .await
            .expect("current sqlite schema should bootstrap");
        drop(bootstrap);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true),
            )
            .await
            .expect("sqlite should open for legacy inventory sync seed");

        sqlx::query("INSERT INTO profiles (profile_id, created_at) VALUES (?1, ?2)")
            .bind("Tavily")
            .bind(1_i64)
            .execute(&pool)
            .await
            .expect("profile row should seed");

        let legacy_import_id = "520ab27d-d226-59e7-9b29-47612c9c4fde";
        let source_url = "https://example.com/tavily.yaml";
        let stable_import_id = ids::stable_import_id(
            &ProxyScope::profile("Tavily").key(),
            &ProxyImportSourceIdentity::from_source(&SubscriptionSource::Url(
                source_url.to_string(),
            ))
            .key(),
        );

        sqlx::query(
            r#"
            INSERT INTO proxy_imports (
              import_id, name, import_kind, source_scope_type, source_scope_profile_id,
              source_type, source_value, allocation_scope_type, allocation_scope_profile_id,
              created_at, updated_at
            )
            VALUES (?1, NULL, 'subscription', 'profile', 'Tavily', 'inventory', ?1, 'profile', 'Tavily', 10, 20)
            "#,
        )
        .bind(legacy_import_id)
        .execute(&pool)
        .await
        .expect("legacy inventory import should seed");

        sqlx::query(
            r#"
            INSERT INTO proxy_inventory_nodes (
              import_id, node_id, source_scope_type, source_scope_profile_id, source_type, source_value,
              allocation_scope_type, allocation_scope_profile_id,
              proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json,
              created_at, updated_at
            )
            VALUES (?1, ?2, 'profile', 'Tavily', 'inventory', ?1, 'profile', 'Tavily', 'tavily-node', 'socks5', '1.1.1.1', '["1.1.1.1"]', '{"name":"tavily-node"}', 10, 20)
            "#,
        )
        .bind(legacy_import_id)
        .bind("11111111-1111-1111-1111-111111111111")
        .execute(&pool)
        .await
        .expect("legacy inventory node should seed");

        sqlx::query(
            r#"
            INSERT INTO proxy_import_sync_configs (
              import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
              last_sync_due_at, last_sync_started_at, last_sync_finished_at,
              last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
              updated_at
            )
            VALUES (?1, 'Tavily', 'url', ?2, 1, 600, 86400, 1, 2, 3, 4, 5, 6, 7)
            "#,
        )
        .bind(legacy_import_id)
        .bind(source_url)
        .execute(&pool)
        .await
        .expect("legacy import-keyed sync config should seed");

        sqlx::query(
            r#"
            INSERT INTO proxy_import_sync_configs (
              import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
              last_sync_due_at, last_sync_started_at, last_sync_finished_at,
              last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
              updated_at
            )
            VALUES (?1, 'Tavily', 'url', ?2, 1, 600, 86400, 11, 12, 13, 14, 15, 16, 17)
            "#,
        )
        .bind(&stable_import_id)
        .bind(source_url)
        .execute(&pool)
        .await
        .expect("stable sync config should seed");

        pool.close().await;

        let store = SqliteStore::open(&path)
            .await
            .expect("sqlite store should keep sync source parseable during short-id migration");

        let configs = store
            .list_proxy_import_sync_configs_for_profile("Tavily")
            .await
            .expect("sync configs should list");
        assert_eq!(
            configs.len(),
            1,
            "legacy import-keyed sync config should merge into stable short-id row"
        );
        assert_eq!(configs[0].import_id, stable_import_id);
        match &configs[0].source {
            SubscriptionSource::Url(value) => assert_eq!(value, source_url),
            other => panic!("unexpected sync source after migration: {other:?}"),
        }
        let imports = store
            .list_proxy_imports()
            .await
            .expect("proxy imports should list");
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_id, stable_import_id);
        assert_eq!(imports[0].source_identity.source_type, "url");
        assert_eq!(imports[0].source_identity.source_value, source_url);
        let nodes = store
            .list_proxy_inventory_for_import(&stable_import_id)
            .await
            .expect("migrated inventory nodes should list");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].import_id, stable_import_id);
        assert_eq!(
            nodes[0].node_id,
            ids::stable_proxy_inventory_node_id(&stable_import_id, "tavily-node")
        );
        assert!(
            store
                .get_proxy_import_sync_config(legacy_import_id)
                .await
                .expect("legacy sync config lookup should succeed")
                .is_none(),
            "legacy inventory-keyed sync config should be removed after merge"
        );

        let _ = tokio::fs::remove_file(path).await;
    }
    #[tokio::test]
    async fn open_merges_partially_migrated_subscription_imports_without_collision() {
        let path = temp_store_path();
        let bootstrap = SqliteStore::open(&path)
            .await
            .expect("current sqlite schema should bootstrap");
        drop(bootstrap);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true),
            )
            .await
            .expect("sqlite should open for partial migration seed");

        sqlx::query("INSERT INTO profiles (profile_id, created_at) VALUES (?1, ?2)")
            .bind("Tavily")
            .bind(1_i64)
            .execute(&pool)
            .await
            .expect("profile row should seed");

        let legacy_import_id = "520ab27d-d226-59e7-9b29-47612c9c4fde";
        let source_url = "https://example.com/tavily.yaml";
        let stable_import_id = ids::stable_import_id(
            &ProxyScope::profile("Tavily").key(),
            &ProxyImportSourceIdentity::from_source(&SubscriptionSource::Url(
                source_url.to_string(),
            ))
            .key(),
        );
        let stable_node_id = ids::stable_proxy_inventory_node_id(&stable_import_id, "tavily-node");

        sqlx::query(
            r#"
            INSERT INTO proxy_imports (
              import_id, name, import_kind, source_scope_type, source_scope_profile_id,
              source_type, source_value, allocation_scope_type, allocation_scope_profile_id,
              created_at, updated_at
            )
            VALUES (?1, NULL, 'subscription', 'profile', 'Tavily', 'inventory', ?1, 'profile', 'Tavily', 10, 20)
            "#,
        )
        .bind(legacy_import_id)
        .execute(&pool)
        .await
        .expect("legacy inventory import should seed");

        sqlx::query(
            r#"
            INSERT INTO proxy_imports (
              import_id, name, import_kind, source_scope_type, source_scope_profile_id,
              source_type, source_value, allocation_scope_type, allocation_scope_profile_id,
              created_at, updated_at
            )
            VALUES (?1, 'Tavily feed', 'subscription', 'profile', 'Tavily', 'url', ?2, 'profile', 'Tavily', 11, 30)
            "#,
        )
        .bind(&stable_import_id)
        .bind(source_url)
        .execute(&pool)
        .await
        .expect("stable short-id import should seed");

        sqlx::query(
            r#"
            INSERT INTO proxy_inventory_nodes (
              import_id, node_id, source_scope_type, source_scope_profile_id, source_type, source_value,
              allocation_scope_type, allocation_scope_profile_id,
              proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json,
              created_at, updated_at
            )
            VALUES (?1, ?2, 'profile', 'Tavily', 'inventory', ?1, 'profile', 'Tavily', 'tavily-node', 'socks5', '1.1.1.1', '["1.1.1.1"]', '{"name":"tavily-node","server":"1.1.1.1"}', 10, 20)
            "#,
        )
        .bind(legacy_import_id)
        .bind("11111111-1111-1111-1111-111111111111")
        .execute(&pool)
        .await
        .expect("legacy inventory node should seed");

        sqlx::query(
            r#"
            INSERT INTO proxy_inventory_nodes (
              import_id, node_id, source_scope_type, source_scope_profile_id, source_type, source_value,
              allocation_scope_type, allocation_scope_profile_id,
              proxy_name, proxy_type, server, resolved_ips_json, raw_proxy_json,
              created_at, updated_at
            )
            VALUES (?1, ?2, 'profile', 'Tavily', 'url', ?3, 'profile', 'Tavily', 'tavily-node', 'socks5', '2.2.2.2', '["2.2.2.2"]', '{"name":"tavily-node","server":"2.2.2.2"}', 11, 30)
            "#,
        )
        .bind(&stable_import_id)
        .bind(&stable_node_id)
        .bind(source_url)
        .execute(&pool)
        .await
        .expect("stable short-id inventory node should seed");

        for import_id in [legacy_import_id, stable_import_id.as_str()] {
            sqlx::query(
                r#"
                INSERT INTO proxy_import_sync_configs (
                  import_id, profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
                  last_sync_due_at, last_sync_started_at, last_sync_finished_at,
                  last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
                  updated_at
                )
                VALUES (?1, 'Tavily', 'url', ?2, 1, 600, 86400, 1, 2, 3, 4, 5, 6, 7)
                "#,
            )
            .bind(import_id)
            .bind(source_url)
            .execute(&pool)
            .await
            .expect("duplicate sync configs should seed");
        }

        pool.close().await;

        let store = SqliteStore::open(&path)
            .await
            .expect("sqlite store should merge partially migrated imports without collision");

        let imports = store
            .list_proxy_imports()
            .await
            .expect("proxy imports should list");
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_id, stable_import_id);
        assert_eq!(imports[0].source_identity.source_type, "url");
        assert_eq!(imports[0].source_identity.source_value, source_url);

        let nodes = store
            .list_proxy_inventory_for_import(&stable_import_id)
            .await
            .expect("merged inventory nodes should list");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, stable_node_id);
        assert_eq!(nodes[0].server, "2.2.2.2");
        assert_eq!(
            nodes[0].resolved_ips,
            vec!["2.2.2.2".to_string(), "1.1.1.1".to_string()]
        );

        let configs = store
            .list_proxy_import_sync_configs_for_profile("Tavily")
            .await
            .expect("sync configs should list");
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].import_id, stable_import_id);

        let _ = tokio::fs::remove_file(path).await;
    }
}
