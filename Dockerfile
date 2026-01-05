# Build gate
FROM golang:1.25-trixie AS gate_builder
WORKDIR /build
COPY ./gate .
RUN GOOS=linux GOARCH=arm64 go build -ldflags="-s -w"
RUN mkdir -p /tmp/dist && cp gate /tmp/dist

# Install cargo-chef
FROM blackdex/rust-musl:aarch64-musl AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /build

# Prepare the build recipe for the secretgen tool
FROM chef AS planner1
COPY ./tools/secretgen .
RUN cargo chef prepare --recipe-path recipe.json

# Build the secretgen tool
FROM chef AS builder1
COPY --from=planner1 /build/recipe.json recipe.json
RUN cargo chef cook --release --target aarch64-unknown-linux-musl --recipe-path recipe.json
COPY ./tools/secretgen .
RUN cargo build --release --target aarch64-unknown-linux-musl
RUN mkdir -p /tmp/dist && cp target/aarch64-unknown-linux-musl/release/secretgen /tmp/dist/secretgen

# Prepare the build recipe for the servers
FROM chef AS planner2
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build the servers
FROM chef AS builder2
COPY --from=planner2 /build/recipe.json recipe.json
RUN cargo chef cook --release --target aarch64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target aarch64-unknown-linux-musl
RUN ["bash", "-O", "extglob", "-c", "mkdir -p /tmp/dist && cp target/aarch64-unknown-linux-musl/release/!(*.*) /tmp/dist || true"]

# Build the runtime image
FROM alpine:3.23 AS runtime
RUN apk update && apk add build-base dumb-init bash sed
WORKDIR /app
COPY ./run .
RUN mkdir -p ./bin
COPY --from=gate_builder /tmp/dist/gate bin/
COPY --from=builder2 /tmp/dist bin/
COPY --from=builder1 /tmp/dist/secretgen /tmp/secretgen
RUN /tmp/secretgen > ./proxy/forwarding.secret
RUN chmod +x ./configure.sh ./start.sh

EXPOSE 25565
CMD "./start.sh"
