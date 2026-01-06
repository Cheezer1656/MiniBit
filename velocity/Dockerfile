FROM golang:1.25-alpine3.23 AS builder
WORKDIR /build
COPY . .
RUN GOOS=linux GOARCH=arm64 go build -ldflags="-s -w"

FROM alpine:3.23
COPY --from=builder /build/gate /bin/gate
WORKDIR /run
EXPOSE 25565
CMD ["/bin/gate"]
