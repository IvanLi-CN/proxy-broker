use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use anyhow::Context;
use async_trait::async_trait;

use crate::{
    models::{
        ApiKeyRecord, IpRecord, ProbeRecord, ProfileProxySettings, ProfileSnapshot,
        ProxyImportRecord, ProxyImportSyncConfig, ProxyInventoryRecord, ProxyNode, ProxyScope,
        SessionRecord, TaskListQuery, TaskRunEventRecord, TaskRunRecord,
    },
    store::BrokerStore,
    tasks::matches_task_query,
};

#[derive(Default)]
struct MemoryStoreState {
    profiles: HashMap<String, ProfileSnapshot>,
    proxy_imports: HashMap<String, ProxyImportRecord>,
    proxy_inventory: HashMap<String, ProxyInventoryRecord>,
    proxy_import_sync_configs: HashMap<String, ProxyImportSyncConfig>,
    profile_proxy_settings: HashMap<String, ProfileProxySettings>,
    api_keys: HashMap<String, ApiKeyRecord>,
}

#[derive(Default, Clone)]
pub struct MemoryStore {
    inner: Arc<RwLock<MemoryStoreState>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn with_profile_mut<R, F>(&self, profile_id: &str, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(&mut ProfileSnapshot) -> R,
    {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let profile = guard.profiles.entry(profile_id.to_string()).or_default();
        Ok(f(profile))
    }

    fn with_profile<R, F>(&self, profile_id: &str, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(&ProfileSnapshot) -> R,
    {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        if let Some(profile) = guard.profiles.get(profile_id) {
            Ok(f(profile))
        } else {
            Ok(f(&ProfileSnapshot::default()))
        }
    }
}

#[async_trait]
impl BrokerStore for MemoryStore {
    async fn list_profiles(&self) -> anyhow::Result<Vec<String>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut profiles = HashSet::new();
        profiles.extend(guard.profiles.keys().cloned());
        profiles.extend(guard.profile_proxy_settings.keys().cloned());
        for config in guard.proxy_import_sync_configs.values() {
            profiles.insert(config.profile_id.clone());
        }
        profiles.extend(
            guard
                .api_keys
                .values()
                .flat_map(|record| record.profile_scope.profile_ids.iter().cloned()),
        );
        for item in guard.proxy_inventory.values() {
            if let Some(profile_id) = item.source_scope.profile_id() {
                profiles.insert(profile_id.to_string());
            }
            if let Some(profile_id) = item.allocation_scope.profile_id() {
                profiles.insert(profile_id.to_string());
            }
        }
        let mut profiles = profiles.into_iter().collect::<Vec<_>>();
        profiles.sort();
        Ok(profiles)
    }

    async fn create_profile(&self, profile_id: &str, _created_at: i64) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        guard.profiles.entry(profile_id.to_string()).or_default();
        guard
            .profile_proxy_settings
            .entry(profile_id.to_string())
            .or_insert(ProfileProxySettings {
                profile_id: profile_id.to_string(),
                use_global_proxies: true,
            });
        Ok(())
    }

    async fn replace_subscription(
        &self,
        profile_id: &str,
        nodes: &[ProxyNode],
    ) -> anyhow::Result<()> {
        self.with_profile_mut(profile_id, |profile| {
            profile.nodes = nodes.to_vec();
        })
        .context("replace subscription failed")?;
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
        self.with_profile_mut(profile_id, |profile| {
            profile.nodes = nodes.to_vec();
            profile.ip_records = ip_records
                .iter()
                .cloned()
                .map(|record| (record.ip.clone(), record))
                .collect();
            profile.probe_records = probe_records.to_vec();
            for session_id in removed_session_ids {
                profile.sessions.remove(session_id);
            }
        })
        .context("apply subscription snapshot failed")?;
        Ok(())
    }

    async fn list_subscription(&self, profile_id: &str) -> anyhow::Result<Vec<ProxyNode>> {
        self.with_profile(profile_id, |profile| profile.nodes.clone())
    }

    async fn list_proxy_inventory(&self) -> anyhow::Result<Vec<ProxyInventoryRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut items = guard.proxy_inventory.values().cloned().collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.proxy_name
                .cmp(&right.proxy_name)
                .then_with(|| left.node_id.cmp(&right.node_id))
        });
        Ok(items)
    }

    async fn replace_proxy_inventory_scope(
        &self,
        source_scope: &ProxyScope,
        nodes: &[ProxyInventoryRecord],
    ) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        guard
            .proxy_inventory
            .retain(|_, item| &item.source_scope != source_scope);
        for node in nodes {
            guard
                .proxy_inventory
                .insert(node.node_id.clone(), node.clone());
        }
        Ok(())
    }

    async fn list_proxy_imports(&self) -> anyhow::Result<Vec<ProxyImportRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut items = guard.proxy_imports.values().cloned().collect::<Vec<_>>();
        items.sort_by(|left, right| left.import_id.cmp(&right.import_id));
        Ok(items)
    }

    async fn get_proxy_import(&self, import_id: &str) -> anyhow::Result<Option<ProxyImportRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(guard.proxy_imports.get(import_id).cloned())
    }

    async fn replace_proxy_inventory_import(
        &self,
        import_record: &ProxyImportRecord,
        nodes: &[ProxyInventoryRecord],
    ) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        guard
            .proxy_imports
            .insert(import_record.import_id.clone(), import_record.clone());
        guard
            .proxy_inventory
            .retain(|_, item| item.import_id != import_record.import_id);
        for node in nodes {
            guard
                .proxy_inventory
                .insert(node.node_id.clone(), node.clone());
        }
        Ok(())
    }

    async fn get_proxy_inventory_node(
        &self,
        node_id: &str,
    ) -> anyhow::Result<Option<ProxyInventoryRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(guard.proxy_inventory.get(node_id).cloned())
    }

    async fn list_proxy_inventory_for_import(
        &self,
        import_id: &str,
    ) -> anyhow::Result<Vec<ProxyInventoryRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut items = guard
            .proxy_inventory
            .values()
            .filter(|item| item.import_id == import_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.proxy_name.cmp(&right.proxy_name));
        Ok(items)
    }

    async fn update_proxy_inventory_allocation(
        &self,
        node_id: &str,
        allocation_scope: &ProxyScope,
        updated_at: i64,
    ) -> anyhow::Result<Option<ProxyInventoryRecord>> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let item = guard.proxy_inventory.get_mut(node_id);
        if let Some(item) = item {
            item.allocation_scope = allocation_scope.clone();
            item.updated_at = updated_at;
            return Ok(Some(item.clone()));
        }
        Ok(None)
    }

    async fn update_proxy_import_allocation(
        &self,
        import_id: &str,
        allocation_scope: &ProxyScope,
        updated_at: i64,
    ) -> anyhow::Result<Option<ProxyImportRecord>> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let Some(import_record) = guard.proxy_imports.get_mut(import_id) else {
            return Ok(None);
        };
        import_record.allocation_scope = allocation_scope.clone();
        import_record.updated_at = updated_at;
        let updated = import_record.clone();
        for item in guard.proxy_inventory.values_mut() {
            if item.import_id == import_id {
                item.allocation_scope = allocation_scope.clone();
                item.updated_at = updated_at;
            }
        }
        Ok(Some(updated))
    }

    async fn delete_proxy_inventory_node(
        &self,
        node_id: &str,
    ) -> anyhow::Result<Option<ProxyInventoryRecord>> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(guard.proxy_inventory.remove(node_id))
    }

    async fn delete_proxy_import(
        &self,
        import_id: &str,
    ) -> anyhow::Result<Option<ProxyImportRecord>> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let removed = guard.proxy_imports.remove(import_id);
        if removed.is_some() {
            guard
                .proxy_inventory
                .retain(|_, item| item.import_id != import_id);
            guard.proxy_import_sync_configs.remove(import_id);
        }
        Ok(removed)
    }

    async fn replace_ip_records(
        &self,
        profile_id: &str,
        records: &[IpRecord],
    ) -> anyhow::Result<()> {
        self.with_profile_mut(profile_id, |profile| {
            let mut next = HashMap::new();
            for record in records {
                next.insert(record.ip.clone(), record.clone());
            }
            profile.ip_records = next;
        })
        .context("replace ip records failed")?;
        Ok(())
    }

    async fn upsert_ip_records(
        &self,
        profile_id: &str,
        records: &[IpRecord],
    ) -> anyhow::Result<()> {
        self.with_profile_mut(profile_id, |profile| {
            for record in records {
                profile.ip_records.insert(record.ip.clone(), record.clone());
            }
        })
        .context("upsert ip records failed")?;
        Ok(())
    }

    async fn list_ip_records(&self, profile_id: &str) -> anyhow::Result<Vec<IpRecord>> {
        self.with_profile(profile_id, |profile| {
            profile.ip_records.values().cloned().collect()
        })
    }

    async fn replace_probe_records(
        &self,
        profile_id: &str,
        records: &[ProbeRecord],
    ) -> anyhow::Result<()> {
        self.with_profile_mut(profile_id, |profile| {
            profile.probe_records = records.to_vec();
        })
        .context("replace probe records failed")?;
        Ok(())
    }

    async fn upsert_probe_records(
        &self,
        profile_id: &str,
        records: &[ProbeRecord],
    ) -> anyhow::Result<()> {
        self.with_profile_mut(profile_id, |profile| {
            let mut index: HashMap<(String, String, String), ProbeRecord> = profile
                .probe_records
                .iter()
                .cloned()
                .map(|r| {
                    (
                        (r.proxy_name.clone(), r.ip.clone(), r.target_url.clone()),
                        r,
                    )
                })
                .collect();
            for record in records {
                index.insert(
                    (
                        record.proxy_name.clone(),
                        record.ip.clone(),
                        record.target_url.clone(),
                    ),
                    record.clone(),
                );
            }
            profile.probe_records = index.into_values().collect();
        })
        .context("upsert probe records failed")?;
        Ok(())
    }

    async fn list_probe_records(&self, profile_id: &str) -> anyhow::Result<Vec<ProbeRecord>> {
        self.with_profile(profile_id, |profile| profile.probe_records.clone())
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
        self.with_profile_mut(profile_id, |profile| {
            for session in sessions {
                profile
                    .sessions
                    .insert(session.session_id.clone(), session.clone());
            }
        })?;
        Ok(())
    }

    async fn insert_sessions_with_touch(
        &self,
        profile_id: &str,
        sessions: &[SessionRecord],
        last_used_at: i64,
    ) -> anyhow::Result<()> {
        self.with_profile_mut(profile_id, |profile| {
            for session in sessions {
                profile
                    .sessions
                    .insert(session.session_id.clone(), session.clone());
            }
            for session in sessions {
                let entry = profile
                    .ip_records
                    .entry(session.selected_ip.clone())
                    .or_insert(IpRecord {
                        ip: session.selected_ip.clone(),
                        country_code: None,
                        country_name: None,
                        region_name: None,
                        city: None,
                        geo_source: None,
                        probe_updated_at: None,
                        geo_updated_at: None,
                        last_used_at: None,
                    });
                entry.last_used_at = Some(last_used_at);
            }
        })?;
        Ok(())
    }

    async fn delete_session(&self, profile_id: &str, session_id: &str) -> anyhow::Result<()> {
        self.with_profile_mut(profile_id, |profile| {
            profile.sessions.remove(session_id);
        })?;
        Ok(())
    }

    async fn list_sessions(&self, profile_id: &str) -> anyhow::Result<Vec<SessionRecord>> {
        self.with_profile(profile_id, |profile| {
            let mut sessions = profile.sessions.values().cloned().collect::<Vec<_>>();
            sessions.sort_by(|a, b| {
                a.created_at
                    .cmp(&b.created_at)
                    .then_with(|| a.session_id.cmp(&b.session_id))
            });
            sessions
        })
    }

    async fn insert_api_key(&self, api_key: &ApiKeyRecord) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        guard
            .api_keys
            .insert(api_key.key_id.clone(), api_key.clone());
        Ok(())
    }

    async fn get_api_key(&self, key_id: &str) -> anyhow::Result<Option<ApiKeyRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(guard.api_keys.get(key_id).cloned())
    }

    async fn list_api_keys(&self, owner_subject: &str) -> anyhow::Result<Vec<ApiKeyRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut api_keys = guard
            .api_keys
            .values()
            .filter(|record| record.created_by_subject == owner_subject)
            .cloned()
            .collect::<Vec<_>>();
        api_keys.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| left.key_id.cmp(&right.key_id))
        });
        Ok(api_keys)
    }

    async fn revoke_api_key(
        &self,
        owner_subject: &str,
        key_id: &str,
        revoked_at: i64,
    ) -> anyhow::Result<bool> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(if let Some(record) = guard.api_keys.get_mut(key_id) {
            if record.created_by_subject == owner_subject {
                record.revoked_at = Some(revoked_at);
                true
            } else {
                false
            }
        } else {
            false
        })
    }

    async fn touch_api_key_last_used(&self, key_id: &str, last_used_at: i64) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        if let Some(record) = guard.api_keys.get_mut(key_id) {
            record.last_used_at = Some(last_used_at);
        }
        Ok(())
    }

    async fn touch_ip_usage(
        &self,
        profile_id: &str,
        ip: &str,
        last_used_at: i64,
    ) -> anyhow::Result<()> {
        self.touch_ip_usages(profile_id, &[ip.to_string()], last_used_at)
            .await
    }

    async fn touch_ip_usages(
        &self,
        profile_id: &str,
        ips: &[String],
        last_used_at: i64,
    ) -> anyhow::Result<()> {
        self.with_profile_mut(profile_id, |profile| {
            for ip in ips {
                let entry = profile
                    .ip_records
                    .entry(ip.to_string())
                    .or_insert(IpRecord {
                        ip: ip.to_string(),
                        country_code: None,
                        country_name: None,
                        region_name: None,
                        city: None,
                        geo_source: None,
                        probe_updated_at: None,
                        geo_updated_at: None,
                        last_used_at: None,
                    });
                entry.last_used_at = Some(last_used_at);
            }
        })?;
        Ok(())
    }

    async fn upsert_proxy_import_sync_config(
        &self,
        config: &ProxyImportSyncConfig,
    ) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        guard
            .proxy_import_sync_configs
            .insert(config.import_id.clone(), config.clone());
        Ok(())
    }

    async fn get_proxy_import_sync_config(
        &self,
        import_id: &str,
    ) -> anyhow::Result<Option<ProxyImportSyncConfig>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(guard.proxy_import_sync_configs.get(import_id).cloned())
    }

    async fn list_proxy_import_sync_configs(&self) -> anyhow::Result<Vec<ProxyImportSyncConfig>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut items = guard
            .proxy_import_sync_configs
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.import_id.cmp(&right.import_id));
        Ok(items)
    }

    async fn list_proxy_import_sync_configs_for_profile(
        &self,
        profile_id: &str,
    ) -> anyhow::Result<Vec<ProxyImportSyncConfig>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut items = guard
            .proxy_import_sync_configs
            .values()
            .filter(|config| config.profile_id == profile_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.import_id.cmp(&right.import_id));
        Ok(items)
    }

    async fn delete_proxy_import_sync_config(&self, import_id: &str) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        guard.proxy_import_sync_configs.remove(import_id);
        Ok(())
    }

    async fn get_profile_proxy_settings(
        &self,
        profile_id: &str,
    ) -> anyhow::Result<Option<ProfileProxySettings>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(guard.profile_proxy_settings.get(profile_id).cloned())
    }

    async fn upsert_profile_proxy_settings(
        &self,
        settings: &ProfileProxySettings,
    ) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        guard
            .profile_proxy_settings
            .insert(settings.profile_id.clone(), settings.clone());
        Ok(())
    }

    async fn insert_task_run(&self, run: &TaskRunRecord) -> anyhow::Result<()> {
        self.with_profile_mut(&run.profile_id, |profile| {
            profile.task_runs.insert(run.run_id.clone(), run.clone());
        })?;
        Ok(())
    }

    async fn update_task_run(&self, run: &TaskRunRecord) -> anyhow::Result<()> {
        self.with_profile_mut(&run.profile_id, |profile| {
            profile.task_runs.insert(run.run_id.clone(), run.clone());
        })?;
        Ok(())
    }

    async fn get_task_run(&self, run_id: &str) -> anyhow::Result<Option<TaskRunRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(guard
            .profiles
            .values()
            .find_map(|profile| profile.task_runs.get(run_id).cloned()))
    }

    async fn list_task_runs(&self, query: &TaskListQuery) -> anyhow::Result<Vec<TaskRunRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut runs = guard
            .profiles
            .values()
            .flat_map(|profile| profile.task_runs.values().cloned())
            .filter(|run| matches_task_query(&run.as_summary(), query))
            .collect::<Vec<_>>();
        runs.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.run_id.cmp(&left.run_id))
        });
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
        self.with_profile_mut(&event.profile_id, |profile| {
            profile
                .task_run_events
                .entry(event.run_id.clone())
                .or_default()
                .push(event.clone());
        })?;
        Ok(())
    }

    async fn list_task_run_events(&self, run_id: &str) -> anyhow::Result<Vec<TaskRunEventRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut events = guard
            .profiles
            .values()
            .find_map(|profile| profile.task_run_events.get(run_id).cloned())
            .unwrap_or_default();
        events.sort_by(|left, right| {
            left.at
                .cmp(&right.at)
                .then_with(|| left.event_id.cmp(&right.event_id))
        });
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryStore;
    use crate::{
        auth::issue_api_key,
        models::{
            ApiKeyProfileScope, ProfileProxySettings, ProxyImportKind, ProxyImportRecord,
            ProxyImportSourceIdentity, ProxyImportSyncConfig, ProxyInventoryRecord, ProxyScope,
            SessionRecord, SubscriptionSource,
        },
        store::BrokerStore,
    };

    #[tokio::test]
    async fn create_profile_persists_empty_profiles_in_list() {
        let store = MemoryStore::new();

        store
            .create_profile("empty-profile", 1)
            .await
            .expect("create should succeed");

        let profiles = store.list_profiles().await.expect("list should succeed");
        assert_eq!(profiles, vec!["empty-profile"]);
    }

    #[tokio::test]
    async fn create_profile_defaults_global_proxy_usage_to_enabled() {
        let store = MemoryStore::new();
        store
            .create_profile("default", 1)
            .await
            .expect("create should succeed");

        let settings = store
            .get_profile_proxy_settings("default")
            .await
            .expect("get should succeed")
            .expect("settings should exist");
        assert!(settings.use_global_proxies);
    }

    #[tokio::test]
    async fn inventory_profiles_are_included_in_profile_catalog() {
        let store = MemoryStore::new();
        store
            .replace_proxy_inventory_scope(
                &ProxyScope::profile("edge-jp"),
                &[ProxyInventoryRecord {
                    import_id: "import-a".to_string(),
                    node_id: "node-a".to_string(),
                    source_scope: ProxyScope::profile("edge-jp"),
                    allocation_scope: ProxyScope::global(),
                    proxy_name: "proxy-a".to_string(),
                    proxy_type: "socks5".to_string(),
                    server: "example.com".to_string(),
                    resolved_ips: vec!["1.1.1.1".to_string()],
                    raw_proxy: serde_json::json!({"name": "proxy-a"}),
                    created_at: 1,
                    updated_at: 1,
                }],
            )
            .await
            .expect("replace should succeed");
        store
            .upsert_profile_proxy_settings(&ProfileProxySettings {
                profile_id: "lab-us".to_string(),
                use_global_proxies: true,
            })
            .await
            .expect("upsert should succeed");

        let profiles = store.list_profiles().await.expect("list should succeed");
        assert_eq!(profiles, vec!["edge-jp", "lab-us"]);
    }

    #[tokio::test]
    async fn list_sessions_is_sorted_by_created_at_then_session_id() {
        let store = MemoryStore::new();
        let profile_id = "memory-sort";
        let sessions = vec![
            SessionRecord {
                session_id: "b".to_string(),
                listen: "127.0.0.1".to_string(),
                port: 18081,
                selected_ip: "1.1.1.1".to_string(),
                proxy_name: "proxy-b".to_string(),
                created_at: 2,
            },
            SessionRecord {
                session_id: "a".to_string(),
                listen: "127.0.0.1".to_string(),
                port: 18080,
                selected_ip: "1.1.1.2".to_string(),
                proxy_name: "proxy-a".to_string(),
                created_at: 2,
            },
            SessionRecord {
                session_id: "c".to_string(),
                listen: "127.0.0.1".to_string(),
                port: 18082,
                selected_ip: "1.1.1.3".to_string(),
                proxy_name: "proxy-c".to_string(),
                created_at: 1,
            },
        ];

        store
            .insert_sessions(profile_id, &sessions)
            .await
            .expect("insert should succeed");

        let listed = store
            .list_sessions(profile_id)
            .await
            .expect("list should succeed");

        let ids = listed
            .into_iter()
            .map(|session| session.session_id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["c", "a", "b"]);
    }

    #[tokio::test]
    async fn delete_proxy_import_removes_in_memory_sync_config() {
        let store = MemoryStore::new();
        let import_id = "imp-Z1x2C3v4B5n6M7q8";

        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: import_id.to_string(),
                    name: Some("edge-jp".to_string()),
                    import_kind: ProxyImportKind::Subscription,
                    source_scope: ProxyScope::profile("edge-jp"),
                    source_identity: ProxyImportSourceIdentity {
                        source_type: "file".to_string(),
                        source_value: "/tmp/edge-jp.yaml".to_string(),
                    },
                    allocation_scope: ProxyScope::profile("edge-jp"),
                    created_at: 1,
                    updated_at: 1,
                },
                &[ProxyInventoryRecord {
                    import_id: import_id.to_string(),
                    node_id: "node-N4m6P8q2R5s7T1u3".to_string(),
                    source_scope: ProxyScope::profile("edge-jp"),
                    allocation_scope: ProxyScope::profile("edge-jp"),
                    proxy_name: "proxy-1".to_string(),
                    proxy_type: "socks5".to_string(),
                    server: "edge.example.com".to_string(),
                    resolved_ips: vec!["1.1.1.1".to_string()],
                    raw_proxy: serde_json::json!({"name": "proxy-1"}),
                    created_at: 1,
                    updated_at: 1,
                }],
            )
            .await
            .expect("import seed should succeed");
        store
            .upsert_proxy_import_sync_config(&ProxyImportSyncConfig {
                import_id: import_id.to_string(),
                profile_id: "edge-jp".to_string(),
                source: SubscriptionSource::File("/tmp/edge-jp.yaml".to_string()),
                enabled: true,
                sync_every_sec: 60,
                full_refresh_every_sec: 600,
                last_sync_due_at: Some(1),
                last_sync_started_at: None,
                last_sync_finished_at: None,
                last_full_refresh_due_at: Some(2),
                last_full_refresh_started_at: None,
                last_full_refresh_finished_at: None,
                updated_at: 1,
            })
            .await
            .expect("sync config seed should succeed");

        store
            .delete_proxy_import(import_id)
            .await
            .expect("delete should succeed")
            .expect("import should exist");

        assert!(
            store
                .get_proxy_import_sync_config(import_id)
                .await
                .expect("sync config query should succeed")
                .is_none()
        );
    }

    #[tokio::test]
    async fn issue_api_key_round_trip() {
        let store = MemoryStore::new();
        let issued = issue_api_key(
            "deploy-bot",
            "admin@example.com",
            ApiKeyProfileScope::selected(["alpha".to_string()]),
        );

        store
            .insert_api_key(&issued.record)
            .await
            .expect("insert should succeed");
        store
            .touch_api_key_last_used(&issued.record.key_id, 42)
            .await
            .expect("touch should succeed");

        let fetched = store
            .get_api_key(&issued.record.key_id)
            .await
            .expect("get should succeed")
            .expect("api key should exist");
        assert_eq!(fetched.secret_hash, issued.record.secret_hash);
        assert_eq!(fetched.last_used_at, Some(42));

        let listed = store
            .list_api_keys("admin@example.com")
            .await
            .expect("list should succeed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "deploy-bot");

        let other_owner_keys = store
            .list_api_keys("viewer@example.com")
            .await
            .expect("list should succeed");
        assert!(other_owner_keys.is_empty());

        let other_owner_revoked = store
            .revoke_api_key("viewer@example.com", &issued.record.key_id, 77)
            .await
            .expect("revoke should succeed");
        assert!(!other_owner_revoked);

        let revoked = store
            .revoke_api_key("admin@example.com", &issued.record.key_id, 88)
            .await
            .expect("revoke should succeed");
        assert!(revoked);

        let revoked_record = store
            .get_api_key(&issued.record.key_id)
            .await
            .expect("get should succeed")
            .expect("api key should still exist");
        assert_eq!(revoked_record.revoked_at, Some(88));
    }
}
