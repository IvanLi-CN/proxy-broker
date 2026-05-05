mod memory;
mod sqlite;

use async_trait::async_trait;

use crate::models::{
    ApiKeyRecord, IpRecord, NodeUsageRecord, ProbeRecord, ProfileProxySettings, ProxyImportRecord,
    ProxyImportSyncConfig, ProxyInventoryRecord, ProxyNode, ProxyNodeMetadataRecord,
    ProxyNodeProbeSampleRecord, ProxyScope, SessionRecord, SystemSettings, TaskListQuery,
    TaskRunEventRecord, TaskRunRecord,
};

pub use memory::MemoryStore;
pub use sqlite::SqliteStore;

#[async_trait]
pub trait BrokerStore: Send + Sync {
    async fn list_profiles(&self) -> anyhow::Result<Vec<String>>;
    async fn create_profile(&self, profile_id: &str, created_at: i64) -> anyhow::Result<()>;

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
    ) -> anyhow::Result<()>;
    async fn list_subscription(&self, profile_id: &str) -> anyhow::Result<Vec<ProxyNode>>;

    async fn list_proxy_inventory(&self) -> anyhow::Result<Vec<ProxyInventoryRecord>>;
    async fn replace_proxy_inventory_scope(
        &self,
        source_scope: &ProxyScope,
        nodes: &[ProxyInventoryRecord],
    ) -> anyhow::Result<()>;
    async fn list_proxy_imports(&self) -> anyhow::Result<Vec<ProxyImportRecord>>;
    async fn get_proxy_import(&self, import_id: &str) -> anyhow::Result<Option<ProxyImportRecord>>;
    async fn replace_proxy_inventory_import(
        &self,
        import_record: &ProxyImportRecord,
        nodes: &[ProxyInventoryRecord],
    ) -> anyhow::Result<()>;
    async fn get_proxy_inventory_node(
        &self,
        node_id: &str,
    ) -> anyhow::Result<Option<ProxyInventoryRecord>>;
    async fn list_proxy_inventory_for_import(
        &self,
        import_id: &str,
    ) -> anyhow::Result<Vec<ProxyInventoryRecord>>;
    async fn update_proxy_inventory_allocation(
        &self,
        node_id: &str,
        allocation_scope: &ProxyScope,
        updated_at: i64,
    ) -> anyhow::Result<Option<ProxyInventoryRecord>>;
    async fn update_proxy_import_allocation(
        &self,
        import_id: &str,
        allocation_scope: &ProxyScope,
        updated_at: i64,
    ) -> anyhow::Result<Option<ProxyImportRecord>>;
    async fn delete_proxy_inventory_node(
        &self,
        node_id: &str,
    ) -> anyhow::Result<Option<ProxyInventoryRecord>>;
    async fn delete_proxy_import(
        &self,
        import_id: &str,
    ) -> anyhow::Result<Option<ProxyImportRecord>>;

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

    async fn upsert_proxy_node_metadata(
        &self,
        records: &[ProxyNodeMetadataRecord],
    ) -> anyhow::Result<()>;
    async fn list_proxy_node_metadata(&self) -> anyhow::Result<Vec<ProxyNodeMetadataRecord>>;
    async fn insert_proxy_node_probe_samples(
        &self,
        records: &[ProxyNodeProbeSampleRecord],
    ) -> anyhow::Result<()>;
    async fn list_recent_proxy_node_probe_samples(
        &self,
        limit_per_node_ip: usize,
    ) -> anyhow::Result<Vec<ProxyNodeProbeSampleRecord>>;

    async fn get_system_settings(&self) -> anyhow::Result<Option<SystemSettings>>;
    async fn upsert_system_settings(&self, settings: &SystemSettings) -> anyhow::Result<()>;

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
    async fn list_profile_node_usages(
        &self,
        profile_id: &str,
    ) -> anyhow::Result<Vec<NodeUsageRecord>>;
    async fn list_session_node_usages(
        &self,
        profile_id: &str,
        session_id: &str,
    ) -> anyhow::Result<Vec<NodeUsageRecord>>;

    async fn insert_api_key(&self, api_key: &ApiKeyRecord) -> anyhow::Result<()>;
    async fn get_api_key(&self, key_id: &str) -> anyhow::Result<Option<ApiKeyRecord>>;
    async fn list_api_keys(&self, owner_subject: &str) -> anyhow::Result<Vec<ApiKeyRecord>>;
    async fn revoke_api_key(
        &self,
        owner_subject: &str,
        key_id: &str,
        revoked_at: i64,
    ) -> anyhow::Result<bool>;
    async fn touch_api_key_last_used(&self, key_id: &str, last_used_at: i64) -> anyhow::Result<()>;

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

    async fn upsert_proxy_import_sync_config(
        &self,
        config: &ProxyImportSyncConfig,
    ) -> anyhow::Result<()>;
    async fn get_proxy_import_sync_config(
        &self,
        import_id: &str,
    ) -> anyhow::Result<Option<ProxyImportSyncConfig>>;
    async fn list_proxy_import_sync_configs(&self) -> anyhow::Result<Vec<ProxyImportSyncConfig>>;
    async fn list_proxy_import_sync_configs_for_profile(
        &self,
        profile_id: &str,
    ) -> anyhow::Result<Vec<ProxyImportSyncConfig>>;
    async fn delete_proxy_import_sync_config(&self, import_id: &str) -> anyhow::Result<()>;

    async fn upsert_profile_sync_config(
        &self,
        config: &ProxyImportSyncConfig,
    ) -> anyhow::Result<()> {
        self.upsert_proxy_import_sync_config(config).await
    }

    async fn get_profile_sync_config(
        &self,
        profile_id: &str,
    ) -> anyhow::Result<Option<ProxyImportSyncConfig>> {
        Ok(self
            .list_proxy_import_sync_configs_for_profile(profile_id)
            .await?
            .into_iter()
            .next())
    }

    async fn list_profile_sync_configs(&self) -> anyhow::Result<Vec<ProxyImportSyncConfig>> {
        self.list_proxy_import_sync_configs().await
    }

    async fn get_profile_proxy_settings(
        &self,
        profile_id: &str,
    ) -> anyhow::Result<Option<ProfileProxySettings>>;
    async fn upsert_profile_proxy_settings(
        &self,
        settings: &ProfileProxySettings,
    ) -> anyhow::Result<()>;

    async fn insert_task_run(&self, run: &TaskRunRecord) -> anyhow::Result<()>;
    async fn update_task_run(&self, run: &TaskRunRecord) -> anyhow::Result<()>;
    async fn get_task_run(&self, run_id: &str) -> anyhow::Result<Option<TaskRunRecord>>;
    async fn list_task_runs(&self, query: &TaskListQuery) -> anyhow::Result<Vec<TaskRunRecord>>;

    async fn insert_task_run_event(&self, event: &TaskRunEventRecord) -> anyhow::Result<()>;
    async fn list_task_run_events(&self, run_id: &str) -> anyhow::Result<Vec<TaskRunEventRecord>>;
}
