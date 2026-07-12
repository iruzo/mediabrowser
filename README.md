<h3 align="center">
    A simple, lightweight web-based file browser
</h3>

<div align="center">
	<img src="./assets/preview.webp"/>
</div>

## Features

- **Apache-style HTTP file serving** - Pure httpd server at root path, compatible with standard tools
- **Enhanced UI** - Simple file browser at `/ui`
- **Upload support** - Upload files up to 256GB
- **TAR downloads** - Download multiple files and directories as TAR
- **File management** - Create folders, delete, modify and upload files
- **File preview** - View images, videos, audio, and text files
- **Recursive search** - Search files and directories recursively from the current UI path

## Usage

### Direct Execution

```bash
cargo run
```

The application runs on **port 30003** with:

- **UI at `/ui`**: `http://localhost:30003/ui`
  - Examples:
    - `http://localhost:30003/ui/` - Root directory in UI
    - `http://localhost:30003/ui/folder/` - Browse folder in UI
    - `http://localhost:30003/ui/folder/file.mp4` - View/play file in UI

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

### UI Routes
- `GET /ui/*` - Server-rendered web interface (gallery for directories, viewer for files)
  - Query parameters: `sort=name|date|size|type`, `filter=all|image|video|audio|text`, `q=term` (recursive search), `offset=n` (large text files)
- `GET /ui/assets/*` - Static UI assets (CSS, JS)
- `POST /ui/form/save|mkdir|rename|delete|download` - Form endpoints used by the web UI

### API Routes
- `GET /api/find?path=folder&query=name` - Recursively list or search paths as JSON strings; directories end with `/`
- `POST /api/upload` - Upload files using multipart `path` and `file` fields (256GB limit)
- `POST /api/downloads` - Stream selected paths as TAR using repeated URL-encoded `path` fields
- `POST /api/rm` - Remove a file or directory using a URL-encoded `path` field
- `POST /api/mkdir` - Recursively create a directory using a URL-encoded `path` field
- `POST /api/write` - Create or replace a UTF-8 text file using URL-encoded `path` and `content` fields
- `POST /api/mv` - Move one path using URL-encoded `from` and `to` fields
- `GET /api/download/path/to/file` - Download single file

### Apache httpd Routes (Root)
- `GET /` - Apache-style directory listing (root)
- `GET /path/to/file` - Direct file access
- `GET /path/to/dir/` - Apache-style directory listing
