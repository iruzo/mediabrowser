FROM docker.io/library/rust:1.94.1-alpine AS builder

RUN apk add --no-cache musl-dev binutils

WORKDIR /app
COPY . .

ENV GRID=1
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN strip /app/target/x86_64-unknown-linux-musl/release/mediabrowser || true

FROM scratch

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/mediabrowser /mediabrowser

EXPOSE 30003

CMD ["/mediabrowser"]
