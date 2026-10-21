FROM rust:1.99.1-slim-bookworm as build
RUN apt-get update && \
    update-ca-certificates && \
    apt-get install --no-install-recommends --assume-yes pkg-config libssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN mkdir -p src/bin

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY src/bin/elasticsearch_exporter.rs src/bin/elasticsearch_exporter.rs

RUN cargo fetch

COPY . .

RUN cargo build --bin elasticsearch_exporter --release

FROM debian:bookworm-slim
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends  \
    ca-certificates=20250419~deb12u2 && \
    update-ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/elasticsearch_exporter /usr/bin/elasticsearch_exporter

ENV RUST_LOG="info"

ENTRYPOINT ["elasticsearch_exporter"]
