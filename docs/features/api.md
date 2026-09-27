# API

The `api` feature enables the file-management API. It is included by the
default `ui` feature.

Build and run it without the default features:

```sh
cargo run --locked --no-default-features --features api
```

API operations use `POST` requests with URL-encoded or multipart form bodies.
The endpoint reference is available in [docs/endpoints](../endpoints/).

The API includes file discovery, reads, downloads, uploads, copies, moves,
removal, and directory creation. Successful mutation requests return `200 OK`
with an empty response body.
