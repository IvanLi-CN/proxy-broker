#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"
STACK_DIR="$REPO_ROOT/deploy/forward-auth"
GENERATED_DIR="${FORWARD_AUTH_GENERATED_DIR:-$STACK_DIR/generated}"

: "${FORWARD_AUTH_DOMAIN_ROOT:=forward-auth.test}"
: "${FORWARD_AUTH_AUTHELIA_HOST:=auth.${FORWARD_AUTH_DOMAIN_ROOT}}"
: "${FORWARD_AUTH_BROKER_HOST:=broker.${FORWARD_AUTH_DOMAIN_ROOT}}"
: "${FORWARD_AUTH_BROKER_BASIC_HOST:=broker-basic.${FORWARD_AUTH_DOMAIN_ROOT}}"
: "${FORWARD_AUTH_MACHINE_HOST:=machine-broker.${FORWARD_AUTH_DOMAIN_ROOT}}"
: "${FORWARD_AUTH_HTTP_PORT:=18080}"
: "${FORWARD_AUTH_HTTPS_PORT:=18443}"
: "${FORWARD_AUTH_ADMIN_GROUP:=proxy-broker-admins}"
: "${FORWARD_AUTH_APP_VERSION:=forward-auth-smoke}"
: "${FORWARD_AUTH_SESSION_SECRET:=$(openssl rand -hex 32)}"
: "${FORWARD_AUTH_STORAGE_ENCRYPTION_KEY:=$(openssl rand -hex 32)}"
: "${FORWARD_AUTH_JWT_SECRET:=$(openssl rand -hex 32)}"

mkdir -p \
  "$GENERATED_DIR/authelia" \
  "$GENERATED_DIR/certs" \
  "$GENERATED_DIR/proxy-broker" \
  "$GENERATED_DIR/traefik/dynamic"

cp "$STACK_DIR/authelia/users_database.yml" "$GENERATED_DIR/authelia/users_database.yml"

TLS_CONFIG="$GENERATED_DIR/certs/openssl-san.cnf"
cat > "$TLS_CONFIG" <<EOF
[req]
distinguished_name = dn
x509_extensions = v3_req
prompt = no

[dn]
CN = ${FORWARD_AUTH_DOMAIN_ROOT}

[v3_req]
subjectAltName = @alt_names

[alt_names]
DNS.1 = ${FORWARD_AUTH_AUTHELIA_HOST}
DNS.2 = ${FORWARD_AUTH_BROKER_HOST}
DNS.3 = ${FORWARD_AUTH_BROKER_BASIC_HOST}
DNS.4 = ${FORWARD_AUTH_MACHINE_HOST}
EOF

openssl req \
  -x509 \
  -nodes \
  -newkey rsa:2048 \
  -keyout "$GENERATED_DIR/certs/forward-auth.key" \
  -out "$GENERATED_DIR/certs/forward-auth.crt" \
  -days 3 \
  -config "$TLS_CONFIG" \
  >/dev/null 2>&1

cat > "$GENERATED_DIR/authelia/configuration.yml" <<EOF
server:
  address: 'tcp://:9091'
  endpoints:
    authz:
      forward-auth:
        implementation: 'ForwardAuth'
        authn_strategies:
          - name: 'HeaderAuthorization'
            schemes:
              - 'Basic'
            scheme_basic_cache_lifespan: 0
          - name: 'CookieSession'

log:
  level: 'info'

identity_validation:
  reset_password:
    jwt_secret: '${FORWARD_AUTH_JWT_SECRET}'

authentication_backend:
  file:
    path: '/config/users_database.yml'

access_control:
  default_policy: 'deny'
  rules:
    - domain: '${FORWARD_AUTH_AUTHELIA_HOST}'
      policy: 'bypass'
    - domain: '${FORWARD_AUTH_BROKER_HOST}'
      policy: 'one_factor'
    - domain: '${FORWARD_AUTH_BROKER_BASIC_HOST}'
      policy: 'one_factor'

session:
  secret: '${FORWARD_AUTH_SESSION_SECRET}'
  cookies:
    - domain: '${FORWARD_AUTH_DOMAIN_ROOT}'
      authelia_url: 'https://${FORWARD_AUTH_AUTHELIA_HOST}:${FORWARD_AUTH_HTTPS_PORT}'
      default_redirection_url: 'https://${FORWARD_AUTH_BROKER_HOST}:${FORWARD_AUTH_HTTPS_PORT}'
      name: 'authelia_session'
      same_site: 'lax'
      inactivity: '10m'
      expiration: '1h'
      remember_me: '1d'

storage:
  encryption_key: '${FORWARD_AUTH_STORAGE_ENCRYPTION_KEY}'
  local:
    path: '/config/db.sqlite3'

notifier:
  filesystem:
    filename: '/config/notification.txt'
EOF

cat > "$GENERATED_DIR/traefik/dynamic/forward-auth.yml" <<EOF
http:
  routers:
    authelia:
      rule: 'Host(\`${FORWARD_AUTH_AUTHELIA_HOST}\`)'
      entryPoints:
        - websecure
      service: authelia
      tls: {}
    proxy-broker:
      rule: 'Host(\`${FORWARD_AUTH_BROKER_HOST}\`)'
      entryPoints:
        - websecure
      middlewares:
        - authelia-session
      service: proxy-broker
      tls: {}
    proxy-broker-basic:
      rule: 'Host(\`${FORWARD_AUTH_BROKER_BASIC_HOST}\`)'
      entryPoints:
        - websecure
      middlewares:
        - authelia-basic
      service: proxy-broker
      tls: {}
    proxy-broker-machine:
      rule: 'Host(\`${FORWARD_AUTH_MACHINE_HOST}\`)'
      entryPoints:
        - websecure
      service: proxy-broker
      tls: {}

  middlewares:
    authelia-session:
      forwardAuth:
        address: 'http://authelia:9091/api/authz/forward-auth'
        trustForwardHeader: true
        authResponseHeaders:
          - 'Remote-User'
          - 'Remote-Groups'
          - 'Remote-Email'
          - 'Remote-Name'
    authelia-basic:
      forwardAuth:
        address: 'http://authelia:9091/api/authz/forward-auth'
        trustForwardHeader: true
        authResponseHeaders:
          - 'Remote-User'
          - 'Remote-Groups'
          - 'Remote-Email'
          - 'Remote-Name'
  services:
    authelia:
      loadBalancer:
        servers:
          - url: 'http://authelia:9091'
    proxy-broker:
      loadBalancer:
        servers:
          - url: 'http://proxy-broker:8080'

tls:
  certificates:
    - certFile: '/etc/traefik/certs/forward-auth.crt'
      keyFile: '/etc/traefik/certs/forward-auth.key'
EOF

cat > "$GENERATED_DIR/stack.env" <<EOF
FORWARD_AUTH_DOMAIN_ROOT=${FORWARD_AUTH_DOMAIN_ROOT}
FORWARD_AUTH_AUTHELIA_HOST=${FORWARD_AUTH_AUTHELIA_HOST}
FORWARD_AUTH_BROKER_HOST=${FORWARD_AUTH_BROKER_HOST}
FORWARD_AUTH_BROKER_BASIC_HOST=${FORWARD_AUTH_BROKER_BASIC_HOST}
FORWARD_AUTH_MACHINE_HOST=${FORWARD_AUTH_MACHINE_HOST}
FORWARD_AUTH_HTTP_PORT=${FORWARD_AUTH_HTTP_PORT}
FORWARD_AUTH_HTTPS_PORT=${FORWARD_AUTH_HTTPS_PORT}
FORWARD_AUTH_ADMIN_GROUP=${FORWARD_AUTH_ADMIN_GROUP}
FORWARD_AUTH_APP_VERSION=${FORWARD_AUTH_APP_VERSION}
FORWARD_AUTH_GENERATED_DIR=${GENERATED_DIR}
FORWARD_AUTH_ADMIN_USER=admin
FORWARD_AUTH_ADMIN_PASSWORD=ProxyBrokerAdmin123!
FORWARD_AUTH_VIEWER_USER=viewer
FORWARD_AUTH_VIEWER_PASSWORD=ProxyBrokerViewer123!
EOF

printf 'Rendered stack config into %s\n' "$GENERATED_DIR"
