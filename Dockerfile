FROM rust:alpine3.20

RUN apk update && apk add build-base dumb-init curl

WORKDIR /build
COPY . .

RUN cargo build --release
RUN mkdir -p /tmp/dist && cp target/release/!(*.*) /tmp/dist/

RUN cd tools/secretgen && cargo build --release


FROM amazoncorretto:22-alpine3.20

RUN apk update && apk add build-base dumb-init curl

WORKDIR /app

COPY ./run .

RUN mkdir -p ./bin
COPY --from=0 /tmp/dist bin/
COPY --from=0 /build/tools/secretgen/target/release/secretgen /tmp/secretgen

RUN /tmp/secretgen > ./proxy/forwarding.secret

RUN mkdir -p ./proxy && curl --output ./proxy/velocity.jar "https://api.papermc.io/v2/projects/velocity/versions/3.3.0-SNAPSHOT/builds/415/downloads/velocity-3.3.0-SNAPSHOT-415.jar"

RUN chmod +x ./start.sh

EXPOSE 25565
CMD ["./start.sh"]