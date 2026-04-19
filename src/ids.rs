use nanoid::nanoid;
use sha2::{Digest, Sha256};

pub const ID_ALPHABET: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
    'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b',
    'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u',
    'v', 'w', 'x', 'y', 'z',
];

pub const ENTITY_ID_BODY_LEN: usize = 16;
pub const SECRET_PART_BODY_LEN: usize = 24;
pub const TEMP_SUFFIX_BODY_LEN: usize = 16;

pub fn random_session_id() -> String {
    prefixed_id("sess", &random_body(ENTITY_ID_BODY_LEN))
}

pub fn random_task_run_id() -> String {
    prefixed_id("run", &random_body(ENTITY_ID_BODY_LEN))
}

pub fn random_task_event_id() -> String {
    prefixed_id("evt", &random_body(ENTITY_ID_BODY_LEN))
}

pub fn random_import_id() -> String {
    prefixed_id("imp", &random_body(ENTITY_ID_BODY_LEN))
}

pub fn random_key_id() -> String {
    prefixed_id("key", &random_body(ENTITY_ID_BODY_LEN))
}

pub fn random_secret_fragment() -> String {
    random_body(SECRET_PART_BODY_LEN)
}

pub fn random_temp_suffix() -> String {
    random_body(TEMP_SUFFIX_BODY_LEN)
}

pub fn stable_import_id(source_scope_key: &str, source_identity_key: &str) -> String {
    stable_prefixed_id(
        "imp",
        "proxy-broker:import",
        &format!("{source_scope_key}:{source_identity_key}"),
    )
}

pub fn stable_proxy_inventory_node_id(import_id: &str, proxy_name: &str) -> String {
    stable_prefixed_id(
        "node",
        "proxy-broker:import-node",
        &format!("{import_id}:{proxy_name}"),
    )
}

pub fn stable_profile_safe_suffix(profile_id: &str) -> String {
    stable_body(
        "proxy-broker:runtime-profile",
        profile_id,
        ENTITY_ID_BODY_LEN,
    )
}

pub fn stable_dedicated_ip_proxy_name(proxy_name: &str, ip: &str) -> String {
    let body = stable_body(
        "proxy-broker:dedicated-ip-proxy",
        &format!("{proxy_name}|{ip}"),
        ENTITY_ID_BODY_LEN,
    );
    format!("broker-ip-{body}")
}

pub fn random_body(len: usize) -> String {
    nanoid!(len, &ID_ALPHABET)
}

pub fn stable_prefixed_id(prefix: &str, namespace: &str, material: &str) -> String {
    prefixed_id(
        prefix,
        &stable_body(namespace, material, ENTITY_ID_BODY_LEN),
    )
}

pub fn stable_body(namespace: &str, material: &str, len: usize) -> String {
    let mut encoded = String::new();
    let mut counter: u32 = 0;
    while encoded.len() < len {
        let mut hasher = Sha256::new();
        hasher.update(namespace.as_bytes());
        hasher.update([0]);
        hasher.update(material.as_bytes());
        hasher.update([0]);
        hasher.update(counter.to_le_bytes());
        encoded.push_str(&encode_base62(&hasher.finalize()));
        counter = counter.wrapping_add(1);
    }
    encoded.truncate(len);
    encoded
}

pub fn is_prefixed_short_id(value: &str, prefix: &str, body_len: usize) -> bool {
    let Some(body) = value
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('-'))
    else {
        return false;
    };
    body.len() == body_len && body.chars().all(|c| ID_ALPHABET.contains(&c))
}

#[cfg(test)]
pub fn is_legacy_uuid_like(value: &str) -> bool {
    let value = value.trim();
    match value.len() {
        32 => value.bytes().all(|byte| byte.is_ascii_hexdigit()),
        36 => value.bytes().enumerate().all(|(idx, byte)| match idx {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        }),
        _ => false,
    }
}

fn prefixed_id(prefix: &str, body: &str) -> String {
    format!("{prefix}-{body}")
}

fn encode_base62(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return ID_ALPHABET[0].to_string();
    }

    let mut digits = vec![0u8];
    for &byte in bytes {
        let mut carry = byte as u32;
        for digit in &mut digits {
            let value = (*digit as u32 * 256) + carry;
            *digit = (value % ID_ALPHABET.len() as u32) as u8;
            carry = value / ID_ALPHABET.len() as u32;
        }
        while carry > 0 {
            digits.push((carry % ID_ALPHABET.len() as u32) as u8);
            carry /= ID_ALPHABET.len() as u32;
        }
    }

    digits
        .iter()
        .rev()
        .map(|digit| ID_ALPHABET[*digit as usize])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_prefixed_ids_use_expected_shape() {
        for id in [
            random_session_id(),
            random_task_run_id(),
            random_task_event_id(),
            random_import_id(),
            random_key_id(),
        ] {
            let (prefix, body) = id.split_once('-').expect("prefixed id should contain dash");
            assert!(["sess", "run", "evt", "imp", "key"].contains(&prefix));
            assert_eq!(body.len(), ENTITY_ID_BODY_LEN);
            assert!(body.chars().all(|c| ID_ALPHABET.contains(&c)));
        }
    }

    #[test]
    fn random_unprefixed_helpers_stay_underscore_safe() {
        for fragment in [random_secret_fragment(), random_temp_suffix()] {
            assert!(!fragment.contains('_'));
            assert!(fragment.chars().all(|c| ID_ALPHABET.contains(&c)));
        }
        assert_eq!(random_secret_fragment().len(), SECRET_PART_BODY_LEN);
        assert_eq!(random_temp_suffix().len(), TEMP_SUFFIX_BODY_LEN);
    }

    #[test]
    fn stable_ids_are_deterministic_and_namespaced() {
        let left = stable_prefixed_id("imp", "ns-a", "same-material");
        let right = stable_prefixed_id("imp", "ns-a", "same-material");
        let other = stable_prefixed_id("imp", "ns-b", "same-material");
        assert_eq!(left, right);
        assert_ne!(left, other);
    }

    #[test]
    fn legacy_uuid_detection_accepts_hyphenated_and_simple_hex() {
        assert!(is_legacy_uuid_like("123e4567-e89b-12d3-a456-426614174000"));
        assert!(is_legacy_uuid_like("123e4567e89b12d3a456426614174000"));
        assert!(!is_legacy_uuid_like("sess-1234567890abcdef"));
    }
}
