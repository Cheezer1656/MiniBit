FROM amazoncorretto:21-alpine3.20-jdk AS builder
WORKDIR /build
COPY . .
RUN chmod +x ./gradlew
RUN ./gradlew build
RUN mkdir -p /tmp/dist && cp ./build/libs/*.jar /tmp/dist

FROM amazoncorretto:22-alpine3.20 AS runtime
RUN apk update && apk add build-base dumb-init curl bash sed
WORKDIR /proxy
RUN mkdir -p /proxy/plugins
RUN curl --output /proxy/velocity.jar "https://api.papermc.io/v2/projects/velocity/versions/3.3.0-SNAPSHOT/builds/415/downloads/velocity-3.3.0-SNAPSHOT-415.jar"
COPY --from=builder /tmp/dist /proxy/plugins/
EXPOSE 25565
CMD ["java", "-jar", "./velocity.jar"]