FROM rust:1-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY tools ./tools

RUN cargo build --release -p inertia-relay

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/inertia-relay /usr/local/bin/inertia-relay

ENV INERTIA_RELAY_DATA_DIR=/data
ENV INERTIA_RELAY_ADDR=0.0.0.0:9000
ENV RUST_LOG=inertia_relay=info,libp2p=info

VOLUME ["/data"]
EXPOSE 9000

ENTRYPOINT ["inertia-relay"]
