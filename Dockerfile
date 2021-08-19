FROM rust:1.54.0 as build

WORKDIR /app

RUN apt-get update \
    && apt-get install --no-install-recommends -y musl-tools=1.1.21-2 \
    && rustup default nightly \
    && rustup target add x86_64-unknown-linux-musl

RUN mkdir -p src/bin

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY src/bin/elasticsearch_exporter.rs src/bin/elasticsearch_exporter.rs

RUN cargo fetch

COPY . .

RUN cargo build --bin elasticsearch_exporter --release --target x86_64-unknown-linux-musl

FROM alpine:3.14.1

COPY --from=build /app/target/x86_64-unknown-linux-musl/release/elasticsearch_exporter /usr/bin/elasticsearch_exporter

ENV RUST_LOG="info"

ENTRYPOINT ["elasticsearch_exporter"]
