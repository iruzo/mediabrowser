<h3 align="center">
    A simple, lightweight file server
</h3>

<div align="center">
	<img src="./assets/preview.webp"/>
</div>

## Usage

### Direct Execution

```bash
cargo run
```

The default build includes the API and UI. Use these commands for each
supported combination:

```bash
# HTTPD only
cargo run --no-default-features --features httpd

# API only
cargo run --no-default-features --features api

# API and UI (default)
cargo run

# HTTPD, API, and UI
cargo run --all-features
```

Use `cargo build --locked --release` for the size-optimized executable.
See [binary size](docs/binary-size.md) for measurements and implementation notes.

The application runs on **port 30003** with:

- **Apache like httpd at root** when built with `httpd`:
  `http://localhost:30003/`
  - Examples:
    - `http://localhost:30003/` - Root directory listing
    - `http://localhost:30003/folder/` - Folder listing
    - `http://localhost:30003/folder/file.mp4` - Direct file access
- **Media browser under `/ui/`** when built with `ui`:
  `http://localhost:30003/ui/`
  - `http://localhost:30003/ui/folder/` browses the same directory as
    `http://localhost:30003/folder/`
  - `http://localhost:30003/ui/folder/file.mp4` obtains the file through
    `POST /api/cat` and opens it in the media viewer

### Docker (Development)

```bash
docker-compose --profile dev up
```

### Docker (Production)

```bash
docker-compose --profile pro up
```

### Docker (HTTPD only)

```bash
docker build --file containers/httpd.Dockerfile --tag mediabrowser-httpd .
```

### Docker (HTTPD + API)

```bash
docker build --file containers/server.Dockerfile --tag mediabrowser-server .
```

### Docker (oneline)
```bash
docker image inspect mediabrowser >/dev/null 2>&1 || docker build -t mediabrowser https://github.com/iruzo/mediabrowser.git && docker run -p 30003:30003 -e BIND_ADDR=0.0.0.0 -v $(pwd)/data:/data mediabrowser
```
```bash
sudo docker image inspect mediabrowser >/dev/null 2>&1 || sudo docker build -t mediabrowser https://github.com/iruzo/mediabrowser.git && sudo docker run -p 30003:30003 -e BIND_ADDR=0.0.0.0 -v $(pwd)/data:/data mediabrowser
```

### Environment Variables

```bash
# Set log level (optional)
export RUST_LOG=debug

# Custom data directory (optional, defaults to /data)
export DATA_DIR=/path/to/your/files

# Bind address (optional, defaults to 127.0.0.1)
# Use 0.0.0.0 to expose to other devices in the network
export BIND_ADDR=127.0.0.1

# Port (optional, defaults to 30003)
export PORT=30003
```

## API Endpoints

Each endpoint is documented with a curl example in
[docs/endpoints/](./docs/endpoints/).

## Security and HTTP Behavior

`DATA_DIR` is the trusted filesystem anchor. Symbolic links below it are hidden
from listings and searches.
Direct paths to a link, or through one, behave as missing paths. Recursive copy
and download skip links; moving or removing a real directory may move or remove
link entries contained by that directory, but their targets are never followed.

UI directory paths end with `/`. A missing trailing slash on a directory is
redirected to its canonical UI path. The UI obtains regular files through
`POST /api/cat`; unsupported media is displayed as the endpoint response.

The UI response is stored and served as gzip. Browsers negotiate this
automatically. Other clients can request and decompress it with:

```bash
curl --compressed http://localhost:30003/ui/
```

A client that explicitly rejects gzip receives `406 Not Acceptable`.

## TODO

- Implement security and user system
- Text file editing (?)
