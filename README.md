<h3 align="center">
    A simple, lightweight web-based file browser
</h3>

<div align="center">
	<img src="./assets/preview.webp"/>
</div>

## Features

- **Apache-style HTTP file serving** - Pure httpd server at root path, compatible with standard tools
- **Enhanced UI** - Modern file browser at `/ui` with advanced features
- **Upload support** - Upload files up to 256GB
- **ZIP downloads** - Download multiple files and directories as ZIP archives
- **File management** - Create folders, delete files
- **File preview** - View images, videos, audio, and text files

## Usage

### Direct Execution

```bash
cargo run
```

The application runs on **port 30003** with:

- **UI at `/ui`**: `http://localhost:30003/ui`
  - Full-featured web interface with uploads, downloads, and file management
  - Browse directories and preview files
  - Examples:
    - `http://localhost:30003/ui/` - Root directory in UI
    - `http://localhost:30003/ui/folder/` - Browse folder in UI
    - `http://localhost:30003/ui/folder/file.mp4` - View/play file in UI

- **Apache like httpd at root**: `http://localhost:30003/`
  - Pure Apache-style file server
  - Compatible with other httpd clients
  - Standard directory listings with no modifications
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
docker image inspect mediabrowser >/dev/null 2>&1 || docker build -t mediabrowser https://github.com/iruzo/mediabrowser.git && docker run -p 30003:30003 -v $(pwd)/data:/data mediabrowser
```
```bash
sudo docker image inspect mediabrowser >/dev/null 2>&1 || sudo docker build -t mediabrowser https://github.com/iruzo/mediabrowser.git && sudo docker run -p 30003:30003 -v $(pwd)/data:/data mediabrowser
```

### Environment Variables

```bash
# Set log level (optional)
export RUST_LOG=debug

# Custom data directory (optional, defaults to /data)
export DATA_DIR=/path/to/your/files
```

## API Endpoints

### UI Routes
- `GET /ui` - Web interface
- `GET /ui/styles.css` - UI stylesheet
- `GET /ui/script.js` - UI JavaScript

### API Routes
- `POST /api/upload?path=/data` - Upload files (multipart form, 256GB limit)
- `GET /api/download-multiple?paths=/data/file1,/data/file2` - Download as ZIP
- `DELETE /api/delete?path=/data/file` - Delete file/directory
- `POST /api/mkdir?path=/data/newfolder` - Create directory
- `GET /api/download?path=/data/file` - Download single file

### Apache httpd Routes (Root)
- `GET /` - Apache-style directory listing (root)
- `GET /path/to/file` - Direct file access
- `GET /path/to/dir/` - Apache-style directory listing
