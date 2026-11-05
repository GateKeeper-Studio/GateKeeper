# Gatekeeper identity provider — production image.
#
# Build from the `server/` directory:
#   docker build -t gatekeeper-server:latest server/
#
# No database is needed at build time: the sqlx compile-time query checks read the
# committed `.sqlx/` cache under SQLX_OFFLINE=1. Regenerate it with
# `cargo sqlx prepare` whenever a query changes (CI verifies it with --check).

# ── Build stage ────────────────────────────────────────────────────────────────
# Pinned to the same toolchain as rust-toolchain.toml. Keep the two in sync;
# bookworm matches the runtime base below so the linked glibc/OpenSSL agree.
FROM rust:1.96-bookworm AS builder

# webauthn-rs links OpenSSL (via webauthn-attestation-ca), so the build needs the
# headers. Everything else in the TLS stack is rustls and needs nothing.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Dependency layer: copy only the manifests first so `cargo build` caches the
# whole dependency graph and is not invalidated by application source edits.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked --bin web-api \
    && rm -rf src

# Real sources. `.sqlx` must be present for the offline query checks.
COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY src ./src

# Bust the fingerprint of our own crate so the stub main.rs above cannot be
# mistaken for the real build.
ENV SQLX_OFFLINE=true
RUN touch src/main.rs && cargo build --release --locked --bin web-api

# ── Runtime stage ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# libssl3 for the OpenSSL the binary links; ca-certificates so outbound TLS
# (SMTP, Google/GitHub token endpoints, webhook delivery) can verify chains;
# curl only so the container healthcheck has something to call.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

# Unprivileged runtime user — nothing here needs root.
RUN useradd --system --uid 10001 --user-group --no-create-home gatekeeper

WORKDIR /app

COPY --from=builder /build/target/release/web-api /usr/local/bin/web-api
# `main()` runs sqlx::migrate!() at boot, which embeds the SQL at compile time —
# these are copied for operator reference (and `sqlx migrate` runs), not required.
COPY --from=builder /build/migrations /app/migrations

USER gatekeeper

# Matches the PORT default in config.rs. Override with -e PORT=... and publish
# whatever you map it to.
EXPOSE 8080

# No shell form: exec form makes the binary PID 1 so it receives SIGTERM directly
# and `with_graceful_shutdown` can drain in-flight requests and the webhook
# dispatcher before exiting.
ENTRYPOINT ["/usr/local/bin/web-api"]
