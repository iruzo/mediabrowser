# POST /api/download

Download the selected paths using repeated URL-encoded `path`
fields (max 1024 paths).

A single regular file is sent as-is. A directory, or any selection
of more than one path, is streamed as TAR.

```html
<form method="post" action="/api/download">
  <label><input type="checkbox" name="path" value="folder" checked>Folder</label>
  <label><input type="checkbox" name="path" value="file.txt" checked>File</label>
  <button type="submit">Download</button>
</form>
```

```sh
curl -X POST \
  -d "path=folder/file.txt" \
  -O -J \
  http://localhost:30003/api/download
```

```sh
curl -X POST \
  -d "path=folder" \
  -d "path=file.txt" \
  -o download.tar \
  http://localhost:30003/api/download
```
