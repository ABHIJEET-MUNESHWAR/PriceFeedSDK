# syntax=docker/dockerfile:1

# ---- builder ----------------------------------------------------------------
FROM rust:1.89-slim AS builder
WORKDIR /build

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests and sources.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY examples-dapp ./examples-dapp

# Build the example watcher binary.
RUN cargo build --release --locked --bin pricefeed-watch \
    && strip target/release/pricefeed-watch

# ---- runtime ----------------------------------------------------------------
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --no-create-home --user-group pricefeed

COPY --from=builder /build/target/release/pricefeed-watch /usr/local/bin/pricefeed-watch

USER 10001:10001

ENV PRICEFEED_ENDPOINT=http://localhost:8080/graphql

ENTRYPOINT ["pricefeed-watch"]
CMD ["--demo"]
