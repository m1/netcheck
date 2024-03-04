# syntax=docker/dockerfile:1
ARG ALPINE_VERSION=3.18
ARG RUST_VERSION=1.76.0

FROM docker.io/library/rust:${RUST_VERSION}-alpine${ALPINE_VERSION} as builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static

WORKDIR /app
COPY . .
RUN cargo build --release

FROM scratch AS runtime

LABEL org.opencontainers.image.title="netcheck" \
      org.opencontainers.image.description="A tool to check network availability" \
      org.opencontainers.image.authors="hello@milescroxford.com" \
      org.opencontainers.image.version="0.0.1"

COPY --from=builder /app/target/release/netcheck .
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

CMD ["./netcheck"]
