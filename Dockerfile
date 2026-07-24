# Zero-dependency Alpine build container for IdleScreen
FROM rust:1.80-alpine as builder

RUN apk add --no-build-cache \
    musl-dev \
    pkgconfig \
    dbus-dev \
    wayland-dev \
    libxkbcommon-dev

WORKDIR /build
COPY . .
RUN cargo build --release -p idle-daemon -p idle-cli

FROM alpine:3.20
RUN apk add --no-build-cache \
    libwayland-client \
    libxkbcommon \
    dbus-libs \
    ca-certificates

COPY --from=builder /build/target/release/idle-daemon /usr/local/bin/
COPY --from=builder /build/target/release/idle /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/idle-daemon"]
