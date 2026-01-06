# Install cargo-chef
FROM blackdex/rust-musl:aarch64-musl AS chef
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
RUN cargo chef cook --release --target aarch64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target aarch64-unknown-linux-musl

# Build the runtime image
FROM alpine:3.23 AS runtime
RUN apk update && apk add build-base dumb-init bash sed
WORKDIR /run
RUN mkdir -p /bin
COPY --from=builder /build/target/aarch64-unknown-linux-musl/release/minibit /bin/minibit

EXPOSE 25565
CMD ["/bin/minibit"]
