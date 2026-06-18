# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY tools ./tools
RUN cargo build --release -p inertia-api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/inertia-api /usr/local/bin/inertia-api
ENV INERTIA_DATA_DIR=/data \
    INERTIA_API_ADDR=0.0.0.0:4783 \
    INERTIA_P2P_ANNOUNCE=/ip4/127.0.0.1/tcp/9002
EXPOSE 4783 9002
VOLUME /data
ENTRYPOINT ["inertia-api"]
