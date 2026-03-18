FROM oven/bun:1.3.10 AS web-build

WORKDIR /app
COPY web ./web
WORKDIR /app/web
RUN bun install --frozen-lockfile && bun run build

FROM rust:1-bookworm AS builder

ARG APP_EFFECTIVE_VERSION
ENV APP_EFFECTIVE_VERSION=${APP_EFFECTIVE_VERSION}

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY --from=web-build /app/web/dist ./web/dist
RUN cargo build --locked --release

FROM debian:bookworm-slim AS runtime

ARG APP_EFFECTIVE_VERSION

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /srv/app
COPY --from=builder /app/target/release/proxy-broker /usr/local/bin/proxy-broker

LABEL org.opencontainers.image.version="${APP_EFFECTIVE_VERSION}"

ENTRYPOINT ["proxy-broker"]
CMD ["--listen", "0.0.0.0:8080", "--session-listen-ip", "0.0.0.0"]
