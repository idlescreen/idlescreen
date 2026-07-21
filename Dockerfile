# Build stage using Alpine Linux
FROM alpine:3.20 AS builder

RUN apk add --no-crate-pkg --no-cache \
    rust cargo build-base pkgconfig dbus-dev wayland-dev libxkbcommon-dev linux-headers

WORKDIR /app
COPY . .

RUN cargo build --release -p trance-daemon -p trance-cli -p trance-tui

# Runtime stage: Alpine Linux preserving ash shell for console & debugging access
FROM alpine:3.20

RUN apk add --no-cache dbus wayland-libs libxkbcommon bash ash

COPY --from=builder /app/target/release/trance-daemon /usr/bin/trance-daemon
COPY --from=builder /app/target/release/trance-cli /usr/bin/trance
COPY --from=builder /app/target/release/trance-tui /usr/bin/trance-tui

ENV WAYLAND_DISPLAY=wayland-0
ENV XDG_CONFIG_HOME=/root/.config

ENTRYPOINT ["/usr/bin/trance-daemon"]
CMD ["--daemon"]
