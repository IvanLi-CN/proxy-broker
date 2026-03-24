use std::path::Path;

use anyhow::Context;
use async_trait::async_trait;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions};

use crate::{
    models::{ApiKeyRecord, IpRecord, ProbeRecord, ProxyNode, SessionRecord},
    store::BrokerStore,
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
