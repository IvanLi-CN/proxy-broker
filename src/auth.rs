use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use axum::{
    extract::{ConnectInfo, FromRequestParts, Request, State},
    http::{HeaderMap, HeaderName, StatusCode, header},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    api::AppState,
    error::BrokerError,
    models::{
        ApiKeyRecord, AuthMeResponse, AuthPrincipalType, CreateApiKeyResponse, now_epoch_sec,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    Enforce,
    Development,
}

impl AuthMode {
    pub fn parse(raw: &str) -> Result<Self, BrokerError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "enforce" => Ok(Self::Enforce),
            "development" => Ok(Self::Development),
            other => Err(BrokerError::InvalidRequest(format!(
                "unsupported auth mode: {other} (expected enforce|development)"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthConfigOptions {
    pub mode: String,
    pub subject_headers: String,
    pub email_headers: String,
    pub groups_headers: String,
    pub trusted_proxies: String,
    pub admin_users: String,
    pub admin_groups: String,
    pub dev_user: String,
    pub dev_email: String,
    pub dev_groups: String,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub mode: AuthMode,
    subject_headers: Vec<HeaderName>,
    email_headers: Vec<HeaderName>,
    groups_headers: Vec<HeaderName>,
    trusted_proxies: Vec<TrustedProxy>,
    admin_users: HashSet<String>,
    admin_groups: HashSet<String>,
    dev_user: String,
    dev_email: Option<String>,
    dev_groups: Vec<String>,
}

impl AuthConfig {
    pub fn from_options(options: AuthConfigOptions) -> Result<Self, BrokerError> {
        let mode = AuthMode::parse(&options.mode)?;
        let dev_user = options.dev_user.trim().to_string();
        if dev_user.is_empty() {
            return Err(BrokerError::InvalidRequest(
                "PROXY_BROKER_AUTH_DEV_USER must not be empty".to_string(),
            ));
        }

        Ok(Self {
            mode,
            subject_headers: parse_header_names(&options.subject_headers, "subject headers")?,
            email_headers: parse_header_names(&options.email_headers, "email headers")?,
            groups_headers: parse_header_names(&options.groups_headers, "groups headers")?,
            trusted_proxies: parse_trusted_proxies(&options.trusted_proxies)?,
            admin_users: parse_value_set(&options.admin_users),
            admin_groups: parse_value_set(&options.admin_groups),
            dev_user,
            dev_email: non_empty_trimmed(&options.dev_email),
            dev_groups: split_csv(&options.dev_groups),
        })
    }

    pub async fn resolve_principal(
        &self,
        headers: &HeaderMap,
        peer_addr: Option<IpAddr>,
        state: &AppState,
    ) -> Result<Option<Principal>, BrokerError> {
        if self.mode == AuthMode::Development {
            return Ok(Some(self.development_principal()));
        }

        let human = self.resolve_human(headers, peer_addr);
        let api_key_secret = extract_api_key_secret(headers)?;

        if human.is_some() && api_key_secret.is_some() {
            return Err(BrokerError::AuthenticationRequired);
        }

        if let Some(candidate) = human {
            return Ok(Some(candidate.into_principal(self)));
        }

        if let Some(secret) = api_key_secret {
            let principal = state.service.authenticate_api_key(&secret).await?;
            return Ok(Some(principal));
        }

        Ok(None)
    }

    pub fn development_principal(&self) -> Principal {
        Principal {
            principal_type: AuthPrincipalType::Development,
            subject: self.dev_user.clone(),
            email: self.dev_email.clone(),
            groups: self.dev_groups.clone(),
            is_admin: true,
            profile_id: None,
            api_key_id: None,
        }
    }

    pub fn is_admin_identity(&self, subject: &str, groups: &[String]) -> bool {
        self.admin_users.contains(subject)
            || groups.iter().any(|group| self.admin_groups.contains(group))
    }

    fn resolve_human(
        &self,
        headers: &HeaderMap,
        peer_addr: Option<IpAddr>,
    ) -> Option<HumanIdentityCandidate> {
        if !self.is_trusted_proxy(peer_addr) {
            return None;
        }

        let subject = find_header_value(headers, &self.subject_headers)?;
        let email = find_header_value(headers, &self.email_headers);
        let groups = self
            .groups_headers
            .iter()
            .find_map(|name| headers.get(name))
            .and_then(|value| value.to_str().ok())
            .map(split_csv)
            .unwrap_or_default();

        Some(HumanIdentityCandidate {
            subject,
            email,
            groups,
        })
    }

    fn is_trusted_proxy(&self, peer_addr: Option<IpAddr>) -> bool {
        let peer_addr = match peer_addr {
            Some(peer_addr) => peer_addr,
            None => return false,
        };

        self.trusted_proxies
            .iter()
            .any(|candidate| candidate.matches(peer_addr))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TrustedProxy {
    V4 { network: u32, prefix_len: u8 },
    V6 { network: u128, prefix_len: u8 },
}

impl TrustedProxy {
    fn parse(raw: &str) -> Result<Self, BrokerError> {
        let (ip_raw, prefix_len_raw) = raw.split_once('/').map_or((raw, None), |(ip, prefix)| {
            (ip, Some(prefix))
        });
        let ip: IpAddr = ip_raw.parse().map_err(|_| {
            BrokerError::InvalidRequest(format!("invalid trusted proxy entry: {raw}"))
        })?;

        match ip {
            IpAddr::V4(ipv4) => {
                let prefix_len = parse_prefix_len(prefix_len_raw, 32, raw)?;
                Ok(Self::V4 {
                    network: mask_v4(ipv4, prefix_len),
                    prefix_len,
                })
            }
            IpAddr::V6(ipv6) => {
                let prefix_len = parse_prefix_len(prefix_len_raw, 128, raw)?;
                Ok(Self::V6 {
                    network: mask_v6(ipv6, prefix_len),
                    prefix_len,
                })
            }
        }
    }

    fn matches(&self, ip: IpAddr) -> bool {
        match (self, ip) {
            (Self::V4 { network, prefix_len }, IpAddr::V4(ipv4)) => {
                *network == mask_v4(ipv4, *prefix_len)
            }
            (Self::V6 { network, prefix_len }, IpAddr::V6(ipv6)) => {
                *network == mask_v6(ipv6, *prefix_len)
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
struct HumanIdentityCandidate {
    subject: String,
    email: Option<String>,
    groups: Vec<String>,
}

impl HumanIdentityCandidate {
    fn into_principal(self, config: &AuthConfig) -> Principal {
        let is_admin = config.is_admin_identity(&self.subject, &self.groups);
        Principal {
            principal_type: AuthPrincipalType::Human,
            subject: self.subject,
            email: self.email,
            groups: self.groups,
            is_admin,
            profile_id: None,
            api_key_id: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Principal {
    pub principal_type: AuthPrincipalType,
    pub subject: String,
    pub email: Option<String>,
    pub groups: Vec<String>,
    pub is_admin: bool,
    pub profile_id: Option<String>,
    pub api_key_id: Option<String>,
}

impl Principal {
    pub fn as_auth_me(&self) -> AuthMeResponse {
        AuthMeResponse {
            authenticated: true,
            principal_type: self.principal_type.clone(),
            subject: self.subject.clone(),
            email: self.email.clone(),
            groups: self.groups.clone(),
            is_admin: self.is_admin,
            profile_id: self.profile_id.clone(),
            api_key_id: self.api_key_id.clone(),
        }
    }

    pub fn api_key(profile_id: String, key_id: String) -> Self {
        Self {
            principal_type: AuthPrincipalType::ApiKey,
            subject: format!("api-key:{key_id}"),
            email: None,
            groups: Vec::new(),
            is_admin: false,
            profile_id: Some(profile_id),
            api_key_id: Some(key_id),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RequestAuthState {
    principal: Option<Principal>,
    error: Option<BrokerError>,
}

impl RequestAuthState {
    pub fn principal(&self) -> Option<&Principal> {
        self.principal.as_ref()
    }

    pub fn require_authenticated(&self) -> Result<&Principal, BrokerError> {
        if let Some(error) = self.error.clone() {
            return Err(error);
        }

        self.principal
            .as_ref()
            .ok_or(BrokerError::AuthenticationRequired)
    }

    pub fn require_admin(&self) -> Result<&Principal, BrokerError> {
        let principal = self.require_authenticated()?;
        if principal.is_admin {
            Ok(principal)
        } else {
            Err(BrokerError::AdminRequired)
        }
    }

    pub fn require_profile_access(&self, profile_id: &str) -> Result<&Principal, BrokerError> {
        let principal = self.require_authenticated()?;
        if principal.is_admin {
            return Ok(principal);
        }

        if principal.principal_type == AuthPrincipalType::ApiKey {
            if principal.profile_id.as_deref() == Some(profile_id) {
                return Ok(principal);
            }
            return Err(BrokerError::ProfileAccessDenied);
        }

        Err(BrokerError::AdminRequired)
    }
}

#[derive(Debug, Clone)]
pub struct AuthContext(pub RequestAuthState);

impl AuthContext {
    pub fn principal(&self) -> Option<&Principal> {
        self.0.principal()
    }

    pub fn require_authenticated(&self) -> Result<&Principal, BrokerError> {
        self.0.require_authenticated()
    }

    pub fn require_admin(&self) -> Result<&Principal, BrokerError> {
        self.0.require_admin()
    }

    pub fn require_profile_access(&self, profile_id: &str) -> Result<&Principal, BrokerError> {
        self.0.require_profile_access(profile_id)
    }
}

impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let auth = parts
            .extensions
            .get::<RequestAuthState>()
            .cloned()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "auth state missing"))?;
        Ok(Self(auth))
    }
}

pub async fn resolve_request_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let peer_addr = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0.ip());
    let auth = match state
        .auth
        .resolve_principal(request.headers(), peer_addr, &state)
        .await
    {
        Ok(principal) => RequestAuthState {
            principal,
            error: None,
        },
        Err(error) => RequestAuthState {
            principal: None,
            error: Some(error),
        },
    };

    request.extensions_mut().insert(auth);
    next.run(request).await
}

#[derive(Debug, Clone)]
pub struct IssuedApiKey {
    pub record: ApiKeyRecord,
    pub secret: String,
}

impl IssuedApiKey {
    pub fn into_response(self) -> CreateApiKeyResponse {
        CreateApiKeyResponse {
            api_key: self.record.as_summary(),
            secret: self.secret,
        }
    }
}

pub fn issue_api_key(profile_id: &str, name: &str, created_by_subject: &str) -> IssuedApiKey {
    let key_id = Uuid::new_v4().simple().to_string();
    let random = Uuid::new_v4().simple().to_string();
    let secret = format!("pbk_{key_id}_{random}");
    let salt = Uuid::new_v4().simple().to_string();
    let created_at = now_epoch_sec();

    IssuedApiKey {
        record: ApiKeyRecord {
            key_id,
            profile_id: profile_id.to_string(),
            name: name.trim().to_string(),
            secret_prefix: secret.chars().take(18).collect(),
            secret_salt: salt.clone(),
            secret_hash: hash_secret(&salt, &secret),
            created_by_subject: created_by_subject.to_string(),
            created_at,
            last_used_at: None,
            revoked_at: None,
        },
        secret,
    }
}

pub fn parse_api_key_secret(secret: &str) -> Result<(&str, &str), BrokerError> {
    let trimmed = secret.trim();
    if !trimmed.starts_with("pbk_") {
        return Err(BrokerError::ApiKeyInvalid);
    }

    let rest = &trimmed[4..];
    let (key_id, _) = rest.split_once('_').ok_or(BrokerError::ApiKeyInvalid)?;
    if key_id.is_empty() {
        return Err(BrokerError::ApiKeyInvalid);
    }

    Ok((key_id, trimmed))
}

pub fn hash_secret(salt: &str, secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(secret.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn constant_time_eq(left: &str, right: &str) -> bool {
    let left_bytes = left.as_bytes();
    let right_bytes = right.as_bytes();
    let max_len = left_bytes.len().max(right_bytes.len());
    let mut diff = left_bytes.len() ^ right_bytes.len();

    for index in 0..max_len {
        let left_byte = left_bytes.get(index).copied().unwrap_or_default();
        let right_byte = right_bytes.get(index).copied().unwrap_or_default();
        diff |= (left_byte ^ right_byte) as usize;
    }

    diff == 0
}

fn parse_header_names(raw: &str, label: &str) -> Result<Vec<HeaderName>, BrokerError> {
    let mut names = Vec::new();
    for value in split_csv(raw) {
        let header = HeaderName::from_bytes(value.as_bytes())
            .map_err(|_| BrokerError::InvalidRequest(format!("invalid {label} entry: {value}")))?;
        names.push(header);
    }
    Ok(names)
}

fn find_header_value(headers: &HeaderMap, names: &[HeaderName]) -> Option<String> {
    names
        .iter()
        .find_map(|name| headers.get(name))
        .and_then(|value| value.to_str().ok())
        .and_then(non_empty_trimmed)
}

fn parse_value_set(raw: &str) -> HashSet<String> {
    split_csv(raw).into_iter().collect()
}

fn parse_trusted_proxies(raw: &str) -> Result<Vec<TrustedProxy>, BrokerError> {
    split_csv(raw)
        .into_iter()
        .map(|entry| TrustedProxy::parse(&entry))
        .collect()
}

fn parse_prefix_len(raw: Option<&str>, max_bits: u8, full_entry: &str) -> Result<u8, BrokerError> {
    match raw {
        Some(raw) => {
            let prefix_len = raw.parse::<u8>().map_err(|_| {
                BrokerError::InvalidRequest(format!("invalid trusted proxy entry: {full_entry}"))
            })?;
            if prefix_len > max_bits {
                return Err(BrokerError::InvalidRequest(format!(
                    "invalid trusted proxy entry: {full_entry}"
                )));
            }
            Ok(prefix_len)
        }
        None => Ok(max_bits),
    }
}

fn mask_v4(ip: Ipv4Addr, prefix_len: u8) -> u32 {
    let raw = u32::from(ip);
    let mask = if prefix_len == 0 {
        0
    } else {
        u32::MAX << (32 - prefix_len)
    };
    raw & mask
}

fn mask_v6(ip: Ipv6Addr, prefix_len: u8) -> u128 {
    let raw = u128::from_be_bytes(ip.octets());
    let mask = if prefix_len == 0 {
        0
    } else {
        u128::MAX << (128 - prefix_len)
    };
    raw & mask
}

fn non_empty_trimmed(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn split_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter_map(non_empty_trimmed)
        .collect::<Vec<_>>()
}

fn extract_api_key_secret(headers: &HeaderMap) -> Result<Option<String>, BrokerError> {
    let authorization_key = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_bearer_secret)
        .and_then(non_empty_trimmed)
        .filter(|value| value.starts_with("pbk_"));

    let direct_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .and_then(non_empty_trimmed);

    match (authorization_key, direct_key) {
        (Some(left), Some(right)) if left != right => Err(BrokerError::AuthenticationRequired),
        (Some(secret), _) => Ok(Some(secret)),
        (_, Some(secret)) => Ok(Some(secret)),
        _ => Ok(None),
    }
}

fn parse_bearer_secret(value: &str) -> Option<&str> {
    let mut parts = value.trim().splitn(2, char::is_whitespace);
    let scheme = parts.next()?;
    let secret = parts.next()?.trim_start();
    if scheme.eq_ignore_ascii_case("bearer") {
        Some(secret)
    } else {
        None
    }
}

pub fn public_path_requires_admin(path: &str) -> bool {
    !(path == "api" || path.starts_with("api/") || path == "healthz")
}

pub fn admin_guard(auth: &AuthContext) -> Result<(), BrokerError> {
    auth.require_admin().map(|_| ())
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header};
    use std::net::{IpAddr, Ipv4Addr};

    use super::{
        AuthConfig, AuthConfigOptions, AuthMode, constant_time_eq, extract_api_key_secret,
    };

    fn sample_config(mode: &str) -> AuthConfig {
        AuthConfig::from_options(AuthConfigOptions {
            mode: mode.to_string(),
            subject_headers: "X-Forwarded-User".to_string(),
            email_headers: "X-Forwarded-Email".to_string(),
            groups_headers: "X-Forwarded-Groups".to_string(),
            trusted_proxies: "127.0.0.1/32,::1/128".to_string(),
            admin_users: "admin@example.com".to_string(),
            admin_groups: "admins".to_string(),
            dev_user: "dev@local".to_string(),
            dev_email: "dev@local".to_string(),
            dev_groups: "proxy-broker-dev-admin".to_string(),
        })
        .expect("config should parse")
    }

    #[test]
    fn auth_mode_parses() {
        assert_eq!(sample_config("enforce").mode, AuthMode::Enforce);
        assert_eq!(sample_config("development").mode, AuthMode::Development);
    }

    #[test]
    fn constant_time_eq_handles_mismatch_lengths() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abcd"));
        assert!(!constant_time_eq("abc", "abd"));
    }

    #[test]
    fn extract_api_key_secret_rejects_conflicting_inputs() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer pbk_left_secret"),
        );
        headers.insert("x-api-key", HeaderValue::from_static("pbk_right_secret"));

        let err = extract_api_key_secret(&headers).expect_err("conflict should fail");
        assert_eq!(err.code(), "authentication_required");
    }

    #[test]
    fn extract_api_key_secret_accepts_lowercase_bearer_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("bearer pbk_left_secret"),
        );

        let secret = extract_api_key_secret(&headers)
            .expect("header should parse")
            .expect("secret should exist");
        assert_eq!(secret, "pbk_left_secret");
    }

    #[test]
    fn human_identity_uses_admin_group_match() {
        let config = sample_config("enforce");
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-user",
            HeaderValue::from_static("user@example.com"),
        );
        headers.insert(
            "x-forwarded-groups",
            HeaderValue::from_static("operators,admins"),
        );

        let human = config
            .resolve_human(&headers, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)))
            .expect("human should resolve");
        let principal = human.into_principal(&config);
        assert!(principal.is_admin);
    }

    #[test]
    fn untrusted_peer_cannot_supply_forwarded_human_identity() {
        let config = sample_config("enforce");
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-user",
            HeaderValue::from_static("admin@example.com"),
        );

        assert!(
            config
                .resolve_human(&headers, Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10))))
                .is_none()
        );
    }
}
