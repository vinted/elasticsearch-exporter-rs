FROM alpine:3.12.1

COPY target/x86_64-unknown-linux-musl/release/elasticsearch_exporter /usr/bin/elasticsearch_exporter

ENV RUST_LOG="info"

ENTRYPOINT ["elasticsearch_exporter"]
