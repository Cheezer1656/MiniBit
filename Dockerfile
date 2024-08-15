# Build Velocity plugin
FROM amazoncorretto:21-alpine3.20-jdk AS jdk_builder
WORKDIR /build
COPY ./velocity .
RUN ./gradlew build
RUN mkdir -p /tmp/dist && cp ./build/libs/*.jar /tmp/dist

# Install cargo-chef
FROM clux/muslrust:stable AS chef
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
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY ./tools/secretgen .
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN mkdir -p /tmp/dist && cp target/x86_64-unknown-linux-musl/release/secretgen /tmp/dist/secretgen

# Prepare the build recipe for the servers
FROM chef AS planner2
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build the servers
FROM chef AS builder2
COPY --from=planner2 /build/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN ["bash", "-O", "extglob", "-c", "mkdir -p /tmp/dist && cp target/x86_64-unknown-linux-musl/release/!(*.*) /tmp/dist || true"]

# Build the runtime image
FROM amazoncorretto:22-alpine3.20 AS runtime
RUN apk update && apk add build-base dumb-init curl bash sed
WORKDIR /app
COPY ./run .
RUN mkdir -p ./proxy/plugins
RUN curl --output ./proxy/velocity.jar "https://api.papermc.io/v2/projects/velocity/versions/3.3.0-SNAPSHOT/builds/415/downloads/velocity-3.3.0-SNAPSHOT-415.jar"
COPY --from=jdk_builder /tmp/dist ./proxy/plugins/
RUN mkdir -p ./bin
COPY --from=builder2 /tmp/dist bin/
COPY --from=builder1 /tmp/dist/secretgen /tmp/secretgen
RUN /tmp/secretgen > ./proxy/forwarding.secret
RUN chmod +x ./start.sh

EXPOSE 25565
CMD "./start.sh"