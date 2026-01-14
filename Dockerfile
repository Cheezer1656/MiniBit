FROM blackdex/rust-musl:aarch64-musl AS rust_musl_arm64
FROM blackdex/rust-musl:x86_64-musl AS rust_musl_amd64

# Install cargo-chef
ARG TARGETARCH
FROM rust_musl_${TARGETARCH} AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /build

# Prepare the build recipe for the servers
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build the servers
FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --target $(uname -m)-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target $(uname -m)-unknown-linux-musl && \
    cp /build/target/$(uname -m)-unknown-linux-musl/release/minibit /build/minibit

# Build the runtime image
FROM alpine:3.23 AS runtime
WORKDIR /run
RUN mkdir -p /bin
COPY --from=builder /build/minibit /bin/minibit

EXPOSE 25565
CMD ["/bin/minibit"]
