## Running

Run from source:

```sh
cargo run --locked
```

Or run the compiled binary:

```sh
./target/release/mediabrowser
```

For containers:

```sh
docker compose --profile pro up --build
```

The default build enables `api` and `ui` over HTTP. The server listens on
`127.0.0.1:30003`; set `BIND_ADDR` and `PORT` to change this. `DATA_DIR` selects
the served directory and defaults to `/data`. Docker Compose mounts `./data`
there and defaults to listening on `0.0.0.0` inside the container.

Open `http://localhost:30003/ui/` to use the browser interface.

## Features and endpoints

Select optional features with Cargo's `--features` flag. Use
`--no-default-features` to disable the default `api` and `ui` features, or
`--all-features` to enable everything. For Docker Compose, set `FEATURES` to
the desired comma-separated list.

- [API](./features/api.md)
- [Browser interface](./features/ui.md)
- [HTTP file server](./features/httpd.md)
- [Authentication and password management](./features/auth.md)
- [HTTPS](./features/https.md)
- [CORS](./features/cors.md)
- [Firewall](./features/firewall.md)
- [Multithreading](./features/multithreading.md)

See the [endpoint documentation](./endpoints/) for individual request formats
and responses.

## Filesystem behavior

`DATA_DIR` is the trusted filesystem anchor. Symbolic links below it are hidden
from listings and searches. Direct paths to a link, or through one, behave as
missing paths. Recursive copy and download skip links; moving or removing a
real directory may move or remove link entries contained by that directory,
but their targets are never followed.

The optional `auth` feature provides directory-based accounts, private user
homes, and sessions held only in memory. Without it, there is no login or
account access control. See [authentication](./features/auth.md) for account
setup and permissions. Endpoint usage is documented under
[login](./endpoints/login.md), [logout](./endpoints/logout.md), and
[password changes](./endpoints/password.md).

## HTTP behavior

UI and HTTPD directory paths end with `/`. A missing trailing slash on a
directory is redirected to its canonical path. The UI obtains regular files
through `POST /api/cat`; unsupported media is displayed as the endpoint response.

The UI response is stored and served as gzip. Browsers negotiate this
automatically. Other clients can request and decompress it with:

```sh
curl --compressed http://localhost:30003/ui/
```

A client that explicitly rejects gzip receives `406 Not Acceptable`.
