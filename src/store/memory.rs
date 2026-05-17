use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use anyhow::Context;
use async_trait::async_trait;

use crate::{
    models::{
        ApiKeyRecord, IpRecord, NodeUsageRecord, ProbeRecord, ProjectProxySettings,
        ProjectSnapshot, ProxyImportRecord, ProxyImportSyncConfig, ProxyInventoryRecord, ProxyNode,
        ProxyNodeMetadataRecord, ProxyNodeProbeSampleRecord, ProxyScope, SessionRecord,
        SystemSettings, TaskListQuery, TaskRunEventRecord, TaskRunRecord,
    },
    store::BrokerStore,
    tasks::matches_task_query,
};

#[derive(Default)]
struct MemoryStoreState {
    projects: HashMap<String, ProjectSnapshot>,
    proxy_imports: HashMap<String, ProxyImportRecord>,
    proxy_inventory: HashMap<String, ProxyInventoryRecord>,
    proxy_node_metadata: HashMap<(String, String), ProxyNodeMetadataRecord>,
    proxy_node_probe_sample_seq: u64,
    proxy_node_probe_samples: Vec<MemoryProxyNodeProbeSample>,
    proxy_import_sync_configs: HashMap<String, ProxyImportSyncConfig>,
    project_proxy_settings: HashMap<String, ProjectProxySettings>,
    system_settings: Option<SystemSettings>,
    api_keys: HashMap<String, ApiKeyRecord>,
}

#[derive(Clone)]
struct MemoryProxyNodeProbeSample {
    seq: u64,
    record: ProxyNodeProbeSampleRecord,
}

#[derive(Default, Clone)]
pub struct MemoryStore {
    inner: Arc<RwLock<MemoryStoreState>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn with_project_mut<R, F>(&self, project_id: &str, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(&mut ProjectSnapshot) -> R,
    {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let project = guard.projects.entry(project_id.to_string()).or_default();
        Ok(f(project))
    }

    fn with_project<R, F>(&self, project_id: &str, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(&ProjectSnapshot) -> R,
    {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        if let Some(project) = guard.projects.get(project_id) {
            Ok(f(project))
        } else {
            Ok(f(&ProjectSnapshot::default()))
        }
    }
}

#[async_trait]
impl BrokerStore for MemoryStore {
    async fn list_projects(&self) -> anyhow::Result<Vec<String>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut projects = HashSet::new();
        projects.extend(guard.projects.keys().cloned());
        projects.extend(guard.project_proxy_settings.keys().cloned());
        for config in guard.proxy_import_sync_configs.values() {
            projects.insert(config.project_id.clone());
        }
        projects.extend(
            guard
                .api_keys
                .values()
                .flat_map(|record| record.project_scope.project_ids.iter().cloned()),
        );
        for item in guard.proxy_inventory.values() {
            if let Some(project_id) = item.source_scope.project_id() {
                projects.insert(project_id.to_string());
            }
            if let Some(project_id) = item.allocation_scope.project_id() {
                projects.insert(project_id.to_string());
            }
        }
        let mut projects = projects.into_iter().collect::<Vec<_>>();
        projects.sort();
        Ok(projects)
    }

    async fn create_project(&self, project_id: &str, _created_at: i64) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        guard.projects.entry(project_id.to_string()).or_default();
        guard
            .project_proxy_settings
            .entry(project_id.to_string())
            .or_insert(ProjectProxySettings {
                project_id: project_id.to_string(),
                use_global_proxies: true,
            });
        Ok(())
    }

    async fn replace_subscription(
        &self,
        project_id: &str,
        nodes: &[ProxyNode],
    ) -> anyhow::Result<()> {
        self.with_project_mut(project_id, |project| {
            project.nodes = nodes.to_vec();
        })
        .context("replace subscription failed")?;
        Ok(())
    }

    async fn apply_subscription_snapshot(
        &self,
        project_id: &str,
        nodes: &[ProxyNode],
        ip_records: &[IpRecord],
        probe_records: &[ProbeRecord],
    ) -> anyhow::Result<()> {
        self.with_project_mut(project_id, |project| {
            project.nodes = nodes.to_vec();
            project.ip_records = ip_records
                .iter()
                .cloned()
                .map(|record| (record.ip.clone(), record))
                .collect();
            project.probe_records = probe_records.to_vec();
        })
        .context("apply subscription snapshot failed")?;
        Ok(())
    }

    async fn list_subscription(&self, project_id: &str) -> anyhow::Result<Vec<ProxyNode>> {
        self.with_project(project_id, |project| project.nodes.clone())
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
        project_id: &str,
        records: &[IpRecord],
    ) -> anyhow::Result<()> {
        self.with_project_mut(project_id, |project| {
            let mut next = HashMap::new();
            for record in records {
                next.insert(record.ip.clone(), record.clone());
            }
            project.ip_records = next;
        })
        .context("replace ip records failed")?;
        Ok(())
    }

    async fn upsert_ip_records(
        &self,
        project_id: &str,
        records: &[IpRecord],
    ) -> anyhow::Result<()> {
        self.with_project_mut(project_id, |project| {
            for record in records {
                project.ip_records.insert(record.ip.clone(), record.clone());
            }
        })
        .context("upsert ip records failed")?;
        Ok(())
    }

    async fn list_ip_records(&self, project_id: &str) -> anyhow::Result<Vec<IpRecord>> {
        self.with_project(project_id, |project| {
            project.ip_records.values().cloned().collect()
        })
    }

    async fn replace_probe_records(
        &self,
        project_id: &str,
        records: &[ProbeRecord],
    ) -> anyhow::Result<()> {
        self.with_project_mut(project_id, |project| {
            project.probe_records = records.to_vec();
        })
        .context("replace probe records failed")?;
        Ok(())
    }

    async fn upsert_probe_records(
        &self,
        project_id: &str,
        records: &[ProbeRecord],
    ) -> anyhow::Result<()> {
        self.with_project_mut(project_id, |project| {
            let mut index: HashMap<(String, String, String), ProbeRecord> = project
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
            project.probe_records = index.into_values().collect();
        })
        .context("upsert probe records failed")?;
        Ok(())
    }

    async fn list_probe_records(&self, project_id: &str) -> anyhow::Result<Vec<ProbeRecord>> {
        self.with_project(project_id, |project| project.probe_records.clone())
    }

    async fn upsert_proxy_node_metadata(
        &self,
        records: &[ProxyNodeMetadataRecord],
    ) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        for record in records {
            guard
                .proxy_node_metadata
                .insert((record.node_id.clone(), record.ip.clone()), record.clone());
        }
        Ok(())
    }

    async fn list_proxy_node_metadata(&self) -> anyhow::Result<Vec<ProxyNodeMetadataRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut items = guard
            .proxy_node_metadata
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.node_id
                .cmp(&right.node_id)
                .then_with(|| left.ip.cmp(&right.ip))
        });
        Ok(items)
    }

    async fn insert_proxy_node_probe_samples(
        &self,
        records: &[ProxyNodeProbeSampleRecord],
    ) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        for record in records {
            guard.proxy_node_probe_sample_seq = guard.proxy_node_probe_sample_seq.saturating_add(1);
            let seq = guard.proxy_node_probe_sample_seq;
            guard
                .proxy_node_probe_samples
                .push(MemoryProxyNodeProbeSample {
                    seq,
                    record: record.clone(),
                });
        }
        guard.proxy_node_probe_samples.sort_by(|left, right| {
            right
                .record
                .sampled_at
                .cmp(&left.record.sampled_at)
                .then_with(|| right.seq.cmp(&left.seq))
                .then_with(|| left.record.node_id.cmp(&right.record.node_id))
                .then_with(|| left.record.ip.cmp(&right.record.ip))
        });

        let mut kept = HashMap::<(String, String), usize>::new();
        guard.proxy_node_probe_samples.retain(|record| {
            let count = kept
                .entry((record.record.node_id.clone(), record.record.ip.clone()))
                .or_insert(0);
            *count += 1;
            *count <= 10
        });
        Ok(())
    }

    async fn list_recent_proxy_node_probe_samples(
        &self,
        limit_per_node_ip: usize,
    ) -> anyhow::Result<Vec<ProxyNodeProbeSampleRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut items = guard.proxy_node_probe_samples.clone();
        items.sort_by(|left, right| {
            right
                .record
                .sampled_at
                .cmp(&left.record.sampled_at)
                .then_with(|| right.seq.cmp(&left.seq))
                .then_with(|| left.record.node_id.cmp(&right.record.node_id))
                .then_with(|| left.record.ip.cmp(&right.record.ip))
        });
        let mut kept = HashMap::<(String, String), usize>::new();
        items.retain(|record| {
            let count = kept
                .entry((record.record.node_id.clone(), record.record.ip.clone()))
                .or_insert(0);
            *count += 1;
            *count <= limit_per_node_ip
        });
        Ok(items.into_iter().map(|item| item.record).collect())
    }

    async fn list_recent_proxy_node_probe_samples_for_pair(
        &self,
        node_id: &str,
        ip: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<ProxyNodeProbeSampleRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut items = guard
            .proxy_node_probe_samples
            .iter()
            .filter(|item| item.record.node_id == node_id && item.record.ip == ip)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .record
                .sampled_at
                .cmp(&left.record.sampled_at)
                .then_with(|| right.seq.cmp(&left.seq))
        });
        items.truncate(limit);
        Ok(items.into_iter().map(|item| item.record).collect())
    }

    async fn get_system_settings(&self) -> anyhow::Result<Option<SystemSettings>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(guard.system_settings.clone())
    }

    async fn upsert_system_settings(&self, settings: &SystemSettings) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        guard.system_settings = Some(settings.clone());
        Ok(())
    }

    async fn insert_session(
        &self,
        project_id: &str,
        session: &SessionRecord,
    ) -> anyhow::Result<()> {
        self.insert_sessions(project_id, std::slice::from_ref(session))
            .await
    }

    async fn insert_sessions(
        &self,
        project_id: &str,
        sessions: &[SessionRecord],
    ) -> anyhow::Result<()> {
        self.with_project_mut(project_id, |project| {
            for session in sessions {
                project
                    .sessions
                    .insert(session.session_id.clone(), session.clone());
            }
        })?;
        Ok(())
    }

    async fn insert_sessions_with_touch(
        &self,
        project_id: &str,
        sessions: &[SessionRecord],
        last_used_at: i64,
    ) -> anyhow::Result<()> {
        self.with_project_mut(project_id, |project| {
            for session in sessions {
                project
                    .sessions
                    .insert(session.session_id.clone(), session.clone());
            }
            for session in sessions {
                let entry = project
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
                project
                    .project_node_usages
                    .insert(session.node_id.clone(), last_used_at);
                project
                    .session_node_usages
                    .entry(session.session_id.clone())
                    .or_default()
                    .insert(session.node_id.clone(), last_used_at);
            }
        })?;
        Ok(())
    }

    async fn delete_session(&self, project_id: &str, session_id: &str) -> anyhow::Result<()> {
        self.with_project_mut(project_id, |project| {
            project.sessions.remove(session_id);
            project.session_node_usages.remove(session_id);
        })?;
        Ok(())
    }

    async fn list_sessions(&self, project_id: &str) -> anyhow::Result<Vec<SessionRecord>> {
        self.with_project(project_id, |project| {
            let mut sessions = project.sessions.values().cloned().collect::<Vec<_>>();
            sessions.sort_by(|a, b| {
                a.created_at
                    .cmp(&b.created_at)
                    .then_with(|| a.session_id.cmp(&b.session_id))
            });
            sessions
        })
    }

    async fn list_project_node_usages(
        &self,
        project_id: &str,
    ) -> anyhow::Result<Vec<NodeUsageRecord>> {
        self.with_project(project_id, |project| {
            let mut usages = project
                .project_node_usages
                .iter()
                .map(|(node_id, last_used_at)| NodeUsageRecord {
                    node_id: node_id.clone(),
                    last_used_at: *last_used_at,
                })
                .collect::<Vec<_>>();
            usages.sort_by(|left, right| {
                right
                    .last_used_at
                    .cmp(&left.last_used_at)
                    .then_with(|| left.node_id.cmp(&right.node_id))
            });
            usages
        })
    }

    async fn list_session_node_usages(
        &self,
        project_id: &str,
        session_id: &str,
    ) -> anyhow::Result<Vec<NodeUsageRecord>> {
        self.with_project(project_id, |project| {
            let mut usages = project
                .session_node_usages
                .get(session_id)
                .into_iter()
                .flat_map(|items| items.iter())
                .map(|(node_id, last_used_at)| NodeUsageRecord {
                    node_id: node_id.clone(),
                    last_used_at: *last_used_at,
                })
                .collect::<Vec<_>>();
            usages.sort_by(|left, right| {
                right
                    .last_used_at
                    .cmp(&left.last_used_at)
                    .then_with(|| left.node_id.cmp(&right.node_id))
            });
            usages
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
        project_id: &str,
        ip: &str,
        last_used_at: i64,
    ) -> anyhow::Result<()> {
        self.touch_ip_usages(project_id, &[ip.to_string()], last_used_at)
            .await
    }

    async fn touch_ip_usages(
        &self,
        project_id: &str,
        ips: &[String],
        last_used_at: i64,
    ) -> anyhow::Result<()> {
        self.with_project_mut(project_id, |project| {
            for ip in ips {
                let entry = project
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

    async fn list_proxy_import_sync_configs_for_project(
        &self,
        project_id: &str,
    ) -> anyhow::Result<Vec<ProxyImportSyncConfig>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut items = guard
            .proxy_import_sync_configs
            .values()
            .filter(|config| config.project_id == project_id)
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

    async fn get_project_proxy_settings(
        &self,
        project_id: &str,
    ) -> anyhow::Result<Option<ProjectProxySettings>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(guard.project_proxy_settings.get(project_id).cloned())
    }

    async fn upsert_project_proxy_settings(
        &self,
        settings: &ProjectProxySettings,
    ) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        guard
            .project_proxy_settings
            .insert(settings.project_id.clone(), settings.clone());
        Ok(())
    }

    async fn insert_task_run(&self, run: &TaskRunRecord) -> anyhow::Result<()> {
        self.with_project_mut(&run.project_id, |project| {
            project.task_runs.insert(run.run_id.clone(), run.clone());
        })?;
        Ok(())
    }

    async fn update_task_run(&self, run: &TaskRunRecord) -> anyhow::Result<()> {
        self.with_project_mut(&run.project_id, |project| {
            project.task_runs.insert(run.run_id.clone(), run.clone());
        })?;
        Ok(())
    }

    async fn get_task_run(&self, run_id: &str) -> anyhow::Result<Option<TaskRunRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(guard
            .projects
            .values()
            .find_map(|project| project.task_runs.get(run_id).cloned()))
    }

    async fn list_task_runs(&self, query: &TaskListQuery) -> anyhow::Result<Vec<TaskRunRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        let mut runs = guard
            .projects
            .values()
            .flat_map(|project| project.task_runs.values().cloned())
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
        self.with_project_mut(&event.project_id, |project| {
            project
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
            .projects
            .values()
            .find_map(|project| project.task_run_events.get(run_id).cloned())
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
            ApiKeyProjectScope, ProjectProxySettings, ProxyImportKind, ProxyImportRecord,
            ProxyImportSourceIdentity, ProxyImportSyncConfig, ProxyInventoryRecord,
            ProxyNodeProbeSampleRecord, ProxyScope, SessionRecord, SubscriptionSource,
        },
        store::BrokerStore,
    };

    #[tokio::test]
    async fn create_project_persists_empty_projects_in_list() {
        let store = MemoryStore::new();

        store
            .create_project("empty-project", 1)
            .await
            .expect("create should succeed");

        let projects = store.list_projects().await.expect("list should succeed");
        assert_eq!(projects, vec!["empty-project"]);
    }

    #[tokio::test]
    async fn create_project_defaults_global_proxy_usage_to_enabled() {
        let store = MemoryStore::new();
        store
            .create_project("default", 1)
            .await
            .expect("create should succeed");

        let settings = store
            .get_project_proxy_settings("default")
            .await
            .expect("get should succeed")
            .expect("settings should exist");
        assert!(settings.use_global_proxies);
    }

    #[tokio::test]
    async fn inventory_projects_are_included_in_project_catalog() {
        let store = MemoryStore::new();
        store
            .replace_proxy_inventory_scope(
                &ProxyScope::project("edge-jp"),
                &[ProxyInventoryRecord {
                    import_id: "import-a".to_string(),
                    node_id: "node-a".to_string(),
                    source_scope: ProxyScope::project("edge-jp"),
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
            .upsert_project_proxy_settings(&ProjectProxySettings {
                project_id: "lab-us".to_string(),
                use_global_proxies: true,
            })
            .await
            .expect("upsert should succeed");

        let projects = store.list_projects().await.expect("list should succeed");
        assert_eq!(projects, vec!["edge-jp", "lab-us"]);
    }

    #[tokio::test]
    async fn list_sessions_is_sorted_by_created_at_then_session_id() {
        let store = MemoryStore::new();
        let project_id = "memory-sort";
        let sessions = vec![
            SessionRecord {
                session_id: "b".to_string(),
                listen: "127.0.0.1".to_string(),
                port: 18081,
                selected_ip: "1.1.1.1".to_string(),
                proxy_name: "proxy-b".to_string(),
                node_id: "node-b".to_string(),
                candidate_node_ids: vec!["node-b".to_string()],
                created_at: 2,
            },
            SessionRecord {
                session_id: "a".to_string(),
                listen: "127.0.0.1".to_string(),
                port: 18080,
                selected_ip: "1.1.1.2".to_string(),
                proxy_name: "proxy-a".to_string(),
                node_id: "node-a".to_string(),
                candidate_node_ids: vec!["node-a".to_string()],
                created_at: 2,
            },
            SessionRecord {
                session_id: "c".to_string(),
                listen: "127.0.0.1".to_string(),
                port: 18082,
                selected_ip: "1.1.1.3".to_string(),
                proxy_name: "proxy-c".to_string(),
                node_id: "node-c".to_string(),
                candidate_node_ids: vec!["node-c".to_string()],
                created_at: 1,
            },
        ];

        store
            .insert_sessions(project_id, &sessions)
            .await
            .expect("insert should succeed");

        let listed = store
            .list_sessions(project_id)
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
                    source_scope: ProxyScope::project("edge-jp"),
                    source_identity: ProxyImportSourceIdentity {
                        source_type: "file".to_string(),
                        source_value: "/tmp/edge-jp.yaml".to_string(),
                    },
                    allocation_scope: ProxyScope::project("edge-jp"),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                &[ProxyInventoryRecord {
                    import_id: import_id.to_string(),
                    node_id: "node-N4m6P8q2R5s7T1u3".to_string(),
                    source_scope: ProxyScope::project("edge-jp"),
                    allocation_scope: ProxyScope::project("edge-jp"),
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
                project_id: "edge-jp".to_string(),
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
    async fn proxy_node_probe_samples_keep_newest_same_second_samples() {
        let store = MemoryStore::new();
        let samples = (0..12)
            .map(|index| ProxyNodeProbeSampleRecord {
                node_id: "node-memory-probe".to_string(),
                ip: "203.0.113.10".to_string(),
                target_url: "https://example.test".to_string(),
                ok: true,
                latency_ms: Some(80 + index),
                sampled_at: 42,
            })
            .collect::<Vec<_>>();

        store
            .insert_proxy_node_probe_samples(&samples)
            .await
            .expect("samples should insert");

        let recent = store
            .list_recent_proxy_node_probe_samples(10)
            .await
            .expect("recent samples should list");
        assert_eq!(recent.len(), 10);
        assert_eq!(recent[0].latency_ms, Some(91));
        assert_eq!(recent[9].latency_ms, Some(82));
    }

    #[tokio::test]
    async fn issue_api_key_round_trip() {
        let store = MemoryStore::new();
        let issued = issue_api_key(
            "deploy-bot",
            "admin@example.com",
            ApiKeyProjectScope::selected(["alpha".to_string()]),
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
