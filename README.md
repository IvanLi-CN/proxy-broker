# proxy-broker

`proxy-broker` is a standalone Rust crate and REST service for:

- loading mihomo subscriptions from URL or file sources,
- building an IP-oriented pool with probe and geo metadata,
- opening and closing local proxy listener sessions by selected IP.

The service binds `127.0.0.1` by default and exposes REST endpoints under `/api/v1`.

## Start the service

```bash
cargo run -- \
  --store sqlite \
  --sqlite-path .proxy-broker/state.sqlite \
  --listen 127.0.0.1:8080
```

## Health check

```bash
curl http://127.0.0.1:8080/healthz
```

## REST API

Base path: `http://127.0.0.1:8080/api/v1/profiles/{profile_id}`

### Load subscription

- `POST /subscriptions/load`
- Request body:

```json
{
  "source": {
    "type": "url",
    "value": "https://example.com/subscription.yaml"
  }
}
```

### Refresh probe and geo metadata

- `POST /refresh`
- Optional request body:

```json
{
  "force": true
}
```

### Extract IPs

- `POST /ips/extract`
- Request body:

```json
{
  "country_codes": ["US", "JP"],
  "cities": ["Tokyo"],
  "specified_ips": ["1.2.3.4"],
  "blacklist_ips": ["5.6.7.8"],
  "limit": 20,
  "sort_mode": "lru"
}
```

### Open one session

- `POST /sessions/open`
- Request body:

```json
{
  "selector": {
    "country_codes": ["JP"],
    "limit": 1,
    "sort_mode": "lru"
  },
  "desired_port": 10080
}
```

### Open sessions in batch

- `POST /sessions/open-batch`

### List sessions

- `GET /sessions`

### Close session

- `DELETE /sessions/{session_id}`

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Contracts

- Library API: `docs/contracts/rust-api.md`
- HTTP API: `docs/contracts/http-apis.md`
- SQLite schema: `docs/contracts/db.md`
- Mihomo config payload: `docs/contracts/file-formats.md`
