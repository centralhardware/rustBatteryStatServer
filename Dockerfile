FROM rust:1.88 AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/BatteryStatServer /usr/local/bin/BatteryStatServer

USER 10001

# /ping answers only when the ClickHouse round trip behind it succeeds.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -fsS "http://localhost:${PORT:-8080}/ping" || exit 1

ENTRYPOINT ["BatteryStatServer"]
