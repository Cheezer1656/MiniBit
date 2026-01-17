FROM amazoncorretto:25-alpine3.22-jdk AS builder
WORKDIR /build
COPY . .
RUN chmod +x ./gradlew
RUN ./gradlew build
RUN mkdir -p /tmp/dist && cp ./build/libs/*.jar /tmp/dist

FROM amazoncorretto:25-alpine3.22 AS runtime
RUN apk update && apk add build-base dumb-init curl bash sed
WORKDIR /proxy
RUN mkdir -p /proxy/plugins
RUN curl --output /proxy/velocity.jar "https://api.papermc.io/v2/projects/velocity/versions/3.3.0-SNAPSHOT/builds/436/downloads/velocity-3.3.0-SNAPSHOT-436.jar" && \
    curl --output /proxy/plugins/viaversion.jar "https://hangarcdn.papermc.io/plugins/ViaVersion/ViaVersion/versions/5.7.1-SNAPSHOT%2B897/PAPER/ViaVersion-5.7.1-SNAPSHOT.jar" && \
    curl --output /proxy/plugins/viabackwards.jar "https://hangarcdn.papermc.io/plugins/ViaVersion/ViaBackwards/versions/5.7.1-SNAPSHOT%2B544/PAPER/ViaBackwards-5.7.1-SNAPSHOT.jar" && \
    curl --output /proxy/plugins/viarewind.jar "https://hangarcdn.papermc.io/plugins/ViaVersion/ViaRewind/versions/4.0.13/PAPER/ViaRewind-4.0.13.jar"
COPY --from=builder /tmp/dist /proxy/plugins/
EXPOSE 25565
CMD ["java", "-jar", "./velocity.jar"]
