use std::path::Path;

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions};
use uuid::Uuid;

use crate::{
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

fn stable_proxy_import_id(
    source_scope: &ProxyScope,
    source_identity: &ProxyImportSourceIdentity,
) -> String {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!(
            "proxy-broker:import:{}:{}",
            source_scope.key(),
            source_identity.key()
        )
        .as_bytes(),
    )
    .to_string()
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
        models::{
            ApiKeyProfileScope, ApiKeyProfileScopeKind, ProxyImportSourceIdentity, ProxyNode,
            ProxyScope, SubscriptionSource,
        },
        store::BrokerStore,
    };
    use sqlx::{Executor, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions};

    fn temp_store_path() -> PathBuf {
        std::env::temp_dir().join(format!("proxy-broker-store-{}.db", uuid::Uuid::new_v4()))
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
        assert_eq!(nodes[0].node_id, "legacy-node");
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
    async fn open_migrates_legacy_single_profile_api_keys() {
        let path =
            std::env::temp_dir().join(format!("proxy-broker-store-{}.db", uuid::Uuid::new_v4()));
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
            .expect("get should succeed")
            .expect("migrated api key should exist");

        assert_eq!(migrated.created_by_subject, "admin@example.com");
        assert_eq!(
            migrated.profile_scope.kind,
            ApiKeyProfileScopeKind::SelectedProfiles
        );
        assert_eq!(migrated.profile_scope.profile_ids, vec!["legacy-profile"]);

        let _ = tokio::fs::remove_file(path).await;
    }
}
