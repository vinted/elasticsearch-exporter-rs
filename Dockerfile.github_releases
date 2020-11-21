FROM alpine:3.12.1 as builder

RUN apk add curl jq wget

RUN curl -s https://api.github.com/repos/vinted/elasticsearch-exporter-rs/releases/latest | jq -r ".assets[] | select(.name | contains(\"musl\")) | .browser_download_url" | wget -i - -O - | tar -xz -C /tmp/

FROM alpine:3.12.1

RUN apk --no-cache add ca-certificates

ENV RUST_LOG="info"

COPY --from=builder /tmp/elasticsearch_exporter /usr/bin/elasticsearch_exporter

ENTRYPOINT ["elasticsearch_exporter"]
