<h3 align="center">
    A simple, lightweight file server
</h3>

<div align="center">
	<img src="./assets/preview.webp"/>
</div>

## Features

- **Apache-style HTTP file serving** - Pure httpd server at root path, compatible with standard tools
- **Upload support** - Upload files up to 256GB
- **TAR downloads** - Download multiple files and directories as TAR
- **File management** - Create folders, copy, move, delete, modify and upload files
- **Recursive search** - Search files and directories recursively from any path

## Usage

### Direct Execution

```bash
cargo run
```

The application runs on **port 30003** with:

- **Apache like httpd at root**: `http://localhost:30003/`
  - Examples:
    - `http://localhost:30003/` - Root directory listing
    - `http://localhost:30003/folder/` - Folder listing
    - `http://localhost:30003/folder/file.mp4` - Direct file access

### Docker (Development)

```bash
docker-compose --profile dev up
```

### Docker (Production)

```bash
docker-compose --profile pro up
```

### Docker (oneline)
```bash
docker image inspect mediabrowser >/dev/null 2>&1 || docker build -t mediabrowser https://github.com/iruzo/mediabrowser.git && docker run -p 30003:30003 -e BIND_ADDR=0.0.0.0 -v $(pwd)/data:/data mediabrowser
```
```bash
sudo docker image inspect mediabrowser >/dev/null 2>&1 || sudo docker build -t mediabrowser https://github.com/iruzo/mediabrowser.git && sudo docker run -p 30003:30003 -e BIND_ADDR=0.0.0.0 -v $(pwd)/data:/data mediabrowser
```

### Build Variants

Which endpoints get compiled into the binary is controlled at build
time by environment variables read in `build.rs`:

- `HTTPD=1` - adds the Apache-style httpd root endpoint
- `GRID=1` - adds httpd plus the `/grid` endpoint
- `UI=1` - adds httpd, grid and the `/ui` endpoint

A Dockerfile is provided per variant in [containers/](./containers/):

- `containers/server.Dockerfile` - API only, no root serving
- `containers/httpd.Dockerfile` - `HTTPD=1`
- `containers/grid.Dockerfile` - `GRID=1`
- `Dockerfile` (root) - `UI=1`, full build

```bash
docker build -t mediabrowser-httpd -f containers/httpd.Dockerfile .
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
[doc/endpoints/](./doc/endpoints/).

### API Routes
- `GET /api/find?path=folder&query=name` - Recursively list or search paths as JSON strings; directories end with `/`
- `POST /api/upload` - Upload files using multipart `path` and `file` fields (256GB limit)
- `POST /api/downloads` - Stream selected paths as TAR using repeated URL-encoded `path` fields
- `POST /api/rm` - Remove a file or directory using a URL-encoded `path` field
- `POST /api/mkdir` - Recursively create a directory using a URL-encoded `path` field
- `POST /api/write` - Create or replace a UTF-8 text file using URL-encoded `path` and `content` fields
- `POST /api/mv` - Move one path using URL-encoded `from` and `to` fields
- `POST /api/cp` - Copy one file or directory using URL-encoded `from` and `to` fields
- `GET /api/download/path/to/file` - Download single file

### Apache httpd Routes (Root)
- `GET /` - Apache-style directory listing (root)
- `GET /path/to/file` - Direct file access
- `GET /path/to/dir/` - Apache-style directory listing
