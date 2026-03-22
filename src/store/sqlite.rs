use std::path::Path;

use anyhow::Context;
use async_trait::async_trait;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions};

use crate::{
    models::{
        ApiKeyRecord, IpRecord, ProbeRecord, ProfileSyncConfig, ProxyNode, SessionRecord,
        SubscriptionSource, TaskEventLevel, TaskListQuery, TaskRunEventRecord, TaskRunKind,
        TaskRunRecord, TaskRunStage, TaskRunStatus, TaskRunTrigger,
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

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
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
        .execute(&self.pool)
        .await?;

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
              SELECT profile_id FROM api_keys
            )
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
        sqlx::query(
            r#"
            INSERT INTO profiles (profile_id, created_at)
            VALUES (?1, ?2)
            "#,
        )
        .bind(profile_id)
        .bind(created_at)
        .execute(&self.pool)
        .await?;
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
        sqlx::query(
            r#"
            INSERT INTO api_keys (
              key_id, profile_id, name, secret_prefix, secret_salt, secret_hash,
              created_by_subject, created_at, last_used_at, revoked_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )
        .bind(&api_key.key_id)
        .bind(&api_key.profile_id)
        .bind(&api_key.name)
        .bind(&api_key.secret_prefix)
        .bind(&api_key.secret_salt)
        .bind(&api_key.secret_hash)
        .bind(&api_key.created_by_subject)
        .bind(api_key.created_at)
        .bind(api_key.last_used_at)
        .bind(api_key.revoked_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_api_key(&self, key_id: &str) -> anyhow::Result<Option<ApiKeyRecord>> {
        let row = sqlx::query(
            r#"
            SELECT key_id, profile_id, name, secret_prefix, secret_salt, secret_hash,
                   created_by_subject, created_at, last_used_at, revoked_at
            FROM api_keys
            WHERE key_id = ?1
            "#,
        )
        .bind(key_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_api_key_row).transpose()
    }

    async fn list_api_keys(&self, profile_id: &str) -> anyhow::Result<Vec<ApiKeyRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT key_id, profile_id, name, secret_prefix, secret_salt, secret_hash,
                   created_by_subject, created_at, last_used_at, revoked_at
            FROM api_keys
            WHERE profile_id = ?1
            ORDER BY created_at DESC, key_id ASC
            "#,
        )
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_api_key_row).collect()
    }

    async fn revoke_api_key(
        &self,
        profile_id: &str,
        key_id: &str,
        revoked_at: i64,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE api_keys
            SET revoked_at = ?3
            WHERE profile_id = ?1 AND key_id = ?2
            "#,
        )
        .bind(profile_id)
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

    async fn upsert_profile_sync_config(&self, config: &ProfileSyncConfig) -> anyhow::Result<()> {
        let (source_type, source_value) = config.source.parts();
        sqlx::query(
            r#"
            INSERT INTO profile_sync_configs (
              profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
              last_sync_due_at, last_sync_started_at, last_sync_finished_at,
              last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
              updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(profile_id) DO UPDATE SET
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

    async fn get_profile_sync_config(
        &self,
        profile_id: &str,
    ) -> anyhow::Result<Option<ProfileSyncConfig>> {
        let row = sqlx::query(
            r#"
            SELECT profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
                   last_sync_due_at, last_sync_started_at, last_sync_finished_at,
                   last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
                   updated_at
            FROM profile_sync_configs
            WHERE profile_id = ?1
            "#,
        )
        .bind(profile_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_profile_sync_config_row).transpose()
    }

    async fn list_profile_sync_configs(&self) -> anyhow::Result<Vec<ProfileSyncConfig>> {
        let rows = sqlx::query(
            r#"
            SELECT profile_id, source_type, source_value, enabled, sync_every_sec, full_refresh_every_sec,
                   last_sync_due_at, last_sync_started_at, last_sync_finished_at,
                   last_full_refresh_due_at, last_full_refresh_started_at, last_full_refresh_finished_at,
                   updated_at
            FROM profile_sync_configs
            ORDER BY profile_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_profile_sync_config_row).collect()
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
        .bind(event.payload_json.as_ref().map(serde_json::to_string).transpose()?)
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

fn map_api_key_row(row: sqlx::sqlite::SqliteRow) -> anyhow::Result<ApiKeyRecord> {
    Ok(ApiKeyRecord {
        key_id: row.try_get("key_id")?,
        profile_id: row.try_get("profile_id")?,
        name: row.try_get("name")?,
        secret_prefix: row.try_get("secret_prefix")?,
        secret_salt: row.try_get("secret_salt")?,
        secret_hash: row.try_get("secret_hash")?,
        created_by_subject: row.try_get("created_by_subject")?,
        created_at: row.try_get("created_at")?,
        last_used_at: row.try_get("last_used_at")?,
        revoked_at: row.try_get("revoked_at")?,
    })
}

fn map_profile_sync_config_row(row: sqlx::sqlite::SqliteRow) -> anyhow::Result<ProfileSyncConfig> {
    let source_type: String = row.try_get("source_type")?;
    let source_value: String = row.try_get("source_value")?;
    let source = SubscriptionSource::from_parts(&source_type, source_value).with_context(|| {
        format!("unsupported profile sync source type: {source_type}")
    })?;
    let sync_every_sec: i64 = row.try_get("sync_every_sec")?;
    let full_refresh_every_sec: i64 = row.try_get("full_refresh_every_sec")?;
    Ok(ProfileSyncConfig {
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
    .bind(run.summary_json.as_ref().map(serde_json::to_string).transpose()?)
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
        kind: TaskRunKind::parse(&kind).with_context(|| format!("unsupported task kind: {kind}"))?,
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
    use super::SqliteStore;
    use crate::{auth::issue_api_key, models::ProxyNode, store::BrokerStore};

    async fn open_temp_store() -> (SqliteStore, std::path::PathBuf) {
        let path =
            std::env::temp_dir().join(format!("proxy-broker-store-{}.db", uuid::Uuid::new_v4()));
        let store = SqliteStore::open(&path)
            .await
            .expect("sqlite store should open");
        (store, path)
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
        let issued = issue_api_key("alpha", "ci-bot", "admin@example.com");

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
            .list_api_keys("alpha")
            .await
            .expect("list should succeed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "ci-bot");

        let revoked = store
            .revoke_api_key("alpha", &issued.record.key_id, 99)
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
}
