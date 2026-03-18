pub const DEFAULT_PROBE_TARGETS: [&str; 2] = [
    "https://www.gstatic.com/generate_204",
    "https://cp.cloudflare.com",
];

pub const DEFAULT_PROBE_TIMEOUT_MS: u64 = 5_000;
pub const DEFAULT_DNS_CONCURRENCY: usize = 32;
pub const DEFAULT_PROBE_CONCURRENCY: usize = 16;
pub const DEFAULT_GEO_ONLINE_CONCURRENCY: usize = 8;

pub const DEFAULT_PROBE_TTL_SEC: u64 = 600;
pub const DEFAULT_GEO_TTL_SEC: u64 = 86_400;

pub const DEFAULT_SERVICE_ADDR: &str = "127.0.0.1:8080";
pub const DEFAULT_SESSION_LISTEN_IP: &str = "127.0.0.1";

pub const DEFAULT_MMDB_URL: &str =
    "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/country.mmdb";
pub const DEFAULT_ONLINE_GEO_BASE: &str = "https://ipwho.is";
