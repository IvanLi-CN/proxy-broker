use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use anyhow::Context;
use async_trait::async_trait;

use crate::{
    models::{ApiKeyRecord, IpRecord, ProbeRecord, ProfileSnapshot, ProxyNode, SessionRecord},
    store::BrokerStore,
};

#[derive(Default, Clone)]
pub struct MemoryStore {
    inner: Arc<RwLock<HashMap<String, ProfileSnapshot>>>,
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
        let profile = guard.entry(profile_id.to_string()).or_default();
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
        if let Some(profile) = guard.get(profile_id) {
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
        let mut profiles = guard.keys().cloned().collect::<Vec<_>>();
        profiles.sort();
        Ok(profiles)
    }

    async fn create_profile(&self, profile_id: &str, _created_at: i64) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        guard.entry(profile_id.to_string()).or_default();
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
        self.with_profile_mut(profile_id, |profile| {
            profile
                .sessions
                .insert(session.session_id.clone(), session.clone());
        })?;
        Ok(())
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
        self.with_profile_mut(&api_key.profile_id, |profile| {
            profile
                .api_keys
                .insert(api_key.key_id.clone(), api_key.clone());
        })?;
        Ok(())
    }

    async fn get_api_key(&self, key_id: &str) -> anyhow::Result<Option<ApiKeyRecord>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        Ok(guard
            .values()
            .find_map(|profile| profile.api_keys.get(key_id).cloned()))
    }

    async fn list_api_keys(&self, profile_id: &str) -> anyhow::Result<Vec<ApiKeyRecord>> {
        self.with_profile(profile_id, |profile| {
            let mut api_keys = profile.api_keys.values().cloned().collect::<Vec<_>>();
            api_keys.sort_by(|left, right| {
                right
                    .created_at
                    .cmp(&left.created_at)
                    .then_with(|| left.key_id.cmp(&right.key_id))
            });
            api_keys
        })
    }

    async fn revoke_api_key(
        &self,
        profile_id: &str,
        key_id: &str,
        revoked_at: i64,
    ) -> anyhow::Result<bool> {
        self.with_profile_mut(profile_id, |profile| {
            if let Some(record) = profile.api_keys.get_mut(key_id) {
                record.revoked_at = Some(revoked_at);
                true
            } else {
                false
            }
        })
    }

    async fn touch_api_key_last_used(&self, key_id: &str, last_used_at: i64) -> anyhow::Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("memory store poisoned"))?;
        for profile in guard.values_mut() {
            if let Some(record) = profile.api_keys.get_mut(key_id) {
                record.last_used_at = Some(last_used_at);
                break;
            }
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
}

#[cfg(test)]
mod tests {
    use super::MemoryStore;
    use crate::{auth::issue_api_key, models::SessionRecord, store::BrokerStore};

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
                session_id: "c".to_string(),
                listen: "127.0.0.1".to_string(),
                port: 18082,
                selected_ip: "1.1.1.2".to_string(),
                proxy_name: "proxy-c".to_string(),
                created_at: 1,
            },
            SessionRecord {
                session_id: "a".to_string(),
                listen: "127.0.0.1".to_string(),
                port: 18080,
                selected_ip: "1.1.1.3".to_string(),
                proxy_name: "proxy-a".to_string(),
                created_at: 1,
            },
        ];
        for session in &sessions {
            store
                .insert_session(profile_id, session)
                .await
                .expect("insert should succeed");
        }

        let listed = store
            .list_sessions(profile_id)
            .await
            .expect("list should succeed");
        let ordered_ids = listed
            .into_iter()
            .map(|item| item.session_id)
            .collect::<Vec<_>>();
        assert_eq!(ordered_ids, vec!["a", "c", "b"]);
    }

    #[tokio::test]
    async fn api_keys_can_be_inserted_listed_and_revoked() {
        let store = MemoryStore::new();
        let issued = issue_api_key("alpha", "deploy-bot", "admin@example.com");

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
        assert_eq!(fetched.last_used_at, Some(42));

        let listed = store
            .list_api_keys("alpha")
            .await
            .expect("list should succeed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "deploy-bot");

        let revoked = store
            .revoke_api_key("alpha", &issued.record.key_id, 88)
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
