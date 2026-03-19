# syntax=docker/dockerfile:1.10

FROM oven/bun:1.3.10 AS web-build

WORKDIR /app/web
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile
COPY web ./
RUN bun run build

FROM rust:1-bookworm AS builder

ARG APP_EFFECTIVE_VERSION
ENV APP_EFFECTIVE_VERSION=${APP_EFFECTIVE_VERSION}

RUN printf 'Acquire::Retries \"5\";\nAcquire::http::Timeout \"30\";\nAcquire::https::Timeout \"30\";\n' > /etc/apt/apt.conf.d/80codex-retries \
    && apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock build.rs ./
COPY --from=web-build /app/web/dist ./web/dist
RUN mkdir src && printf 'fn main() {}\n' > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --locked --release
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --locked --release \
    && install -Dm755 /app/target/release/proxy-broker /tmp/proxy-broker

FROM debian:bookworm-slim AS runtime

ARG APP_EFFECTIVE_VERSION

RUN printf 'Acquire::Retries \"5\";\nAcquire::http::Timeout \"30\";\nAcquire::https::Timeout \"30\";\n' > /etc/apt/apt.conf.d/80codex-retries \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /srv/app
COPY --from=builder /tmp/proxy-broker /usr/local/bin/proxy-broker

LABEL org.opencontainers.image.version="${APP_EFFECTIVE_VERSION}"

ENTRYPOINT ["proxy-broker"]
CMD ["--listen", "0.0.0.0:8080", "--session-listen-ip", "0.0.0.0"]
