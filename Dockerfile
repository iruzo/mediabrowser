FROM rust:1.82-alpine as builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY . .

ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/filemanager /filemanager

EXPOSE 30003

CMD ["/filemanager"]
