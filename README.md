<h3 align="center">
    A simple, lightweight file server
</h3>

<div align="center">
	<img src="./assets/preview.webp"/>
</div>

## Usage

### Run

```sh
cargo run
```

The default build enables `api` and `ui` over HTTP. Enable other combinations with
Cargo features; see the [feature documentation](./docs/features/).

```sh
# All features, including HTTPS, CORS, and firewall
cargo run --all-features

# Release build
cargo build --locked --release
```

The server listens on `127.0.0.1:30003`. Set `BIND_ADDR` and `PORT`.

For containers: `docker compose --profile pro up --build`.

See the [endpoint documentation](./docs/endpoints/) for details.

## Security and HTTP Behavior

`DATA_DIR` is the trusted filesystem anchor. Symbolic links below it are hidden
from listings and searches.
Direct paths to a link, or through one, behave as missing paths. Recursive copy
and download skip links; moving or removing a real directory may move or remove
link entries contained by that directory, but their targets are never followed.

UI and HTTPD directory paths end with `/`. A missing trailing slash on a
directory is redirected to its canonical path. The UI obtains regular files through
`POST /api/cat`; unsupported media is displayed as the endpoint response.

The UI response is stored and served as gzip. Browsers negotiate this
automatically. Other clients can request and decompress it with:

```bash
curl --compressed http://localhost:30003/ui/
```

A client that explicitly rejects gzip receives `406 Not Acceptable`.

## TODO

- User management
- Text file editing (?)
