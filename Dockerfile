FROM docker.io/library/alpine:3.24.1 AS minifier

RUN apk add --no-cache minify=2.24.17-r0

COPY src/endpoints/ui/index.html /static/
COPY src/endpoints/ui/style.css /static/
COPY src/endpoints/ui/script.js /static/
RUN find /static -type f \( -name '*.html' -o -name '*.css' -o -name '*.js' \) \
    -exec minify -o {} {} \;

FROM docker.io/library/rust:1.94.1-alpine AS builder

RUN apk add --no-cache musl-dev binutils

WORKDIR /app
COPY . .
COPY --from=minifier /static/index.html ./src/endpoints/ui/index.html
COPY --from=minifier /static/style.css ./src/endpoints/ui/style.css
COPY --from=minifier /static/script.js ./src/endpoints/ui/script.js

# ARG FEATURES

ENV RUSTFLAGS="-C target-feature=+crt-static"
# RUN cargo build --release --target x86_64-unknown-linux-musl ${FEATURES:+--features "$FEATURES"}
RUN cargo build --locked --release --target x86_64-unknown-linux-musl
RUN strip /app/target/x86_64-unknown-linux-musl/release/mediabrowser || true

FROM scratch

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/mediabrowser /mediabrowser

EXPOSE 30003

CMD ["/mediabrowser"]
