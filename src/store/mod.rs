mod memory;
mod sqlite;

use async_trait::async_trait;

use crate::models::{IpRecord, ProbeRecord, ProxyNode, SessionRecord};

pub use memory::MemoryStore;
pub use sqlite::SqliteStore;

#[async_trait]
pub trait BrokerStore: Send + Sync {
    async fn list_profiles(&self) -> anyhow::Result<Vec<String>>;

    async fn replace_subscription(
        &self,
        profile_id: &str,
        nodes: &[ProxyNode],
    ) -> anyhow::Result<()>;
    async fn apply_subscription_snapshot(
        &self,
        profile_id: &str,
        nodes: &[ProxyNode],
        ip_records: &[IpRecord],
        probe_records: &[ProbeRecord],
        removed_session_ids: &[String],
    ) -> anyhow::Result<()>;
    async fn list_subscription(&self, profile_id: &str) -> anyhow::Result<Vec<ProxyNode>>;

    async fn replace_ip_records(
        &self,
        profile_id: &str,
        records: &[IpRecord],
    ) -> anyhow::Result<()>;
    async fn upsert_ip_records(&self, profile_id: &str, records: &[IpRecord])
    -> anyhow::Result<()>;
    async fn list_ip_records(&self, profile_id: &str) -> anyhow::Result<Vec<IpRecord>>;

    async fn replace_probe_records(
        &self,
        profile_id: &str,
        records: &[ProbeRecord],
    ) -> anyhow::Result<()>;
    async fn upsert_probe_records(
        &self,
        profile_id: &str,
        records: &[ProbeRecord],
    ) -> anyhow::Result<()>;
    async fn list_probe_records(&self, profile_id: &str) -> anyhow::Result<Vec<ProbeRecord>>;

    async fn insert_session(&self, profile_id: &str, session: &SessionRecord)
    -> anyhow::Result<()>;
    async fn insert_sessions(
        &self,
        profile_id: &str,
        sessions: &[SessionRecord],
    ) -> anyhow::Result<()>;
    async fn insert_sessions_with_touch(
        &self,
        profile_id: &str,
        sessions: &[SessionRecord],
        last_used_at: i64,
    ) -> anyhow::Result<()>;
    async fn delete_session(&self, profile_id: &str, session_id: &str) -> anyhow::Result<()>;
    async fn list_sessions(&self, profile_id: &str) -> anyhow::Result<Vec<SessionRecord>>;

    async fn touch_ip_usage(
        &self,
        profile_id: &str,
        ip: &str,
        last_used_at: i64,
    ) -> anyhow::Result<()>;
    async fn touch_ip_usages(
        &self,
        profile_id: &str,
        ips: &[String],
        last_used_at: i64,
    ) -> anyhow::Result<()>;
}
