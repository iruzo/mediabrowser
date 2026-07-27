FROM docker.io/library/alpine AS minifier

RUN apk add --no-cache minify

COPY src/endpoints/grid/grid.html /static/
RUN find /static -type f \( -name '*.html' -o -name '*.css' -o -name '*.js' \) \
    -exec minify -o {} {} \;

FROM docker.io/library/rust:1.94.1-alpine AS builder

RUN apk add --no-cache musl-dev binutils

WORKDIR /app
COPY . .
RUN rm ./src/endpoints/grid/grid.html
COPY --from=minifier /static/grid.html ./src/endpoints/grid/grid.html

# ARG FEATURES

ENV RUSTFLAGS="-C target-feature=+crt-static"
# RUN cargo build --release --target x86_64-unknown-linux-musl ${FEATURES:+--features "$FEATURES"}
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN strip /app/target/x86_64-unknown-linux-musl/release/mediabrowser || true

FROM scratch

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/mediabrowser /mediabrowser

EXPOSE 30003

CMD ["/mediabrowser"]
