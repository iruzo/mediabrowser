# POST /api/upload

Upload files using multipart `path` and `file` fields (256GB limit).
The `path` field must precede the `file` fields. Colliding names
get a numeric suffix before the extension.

```sh
curl -X POST \
  -F "path=folder" \
  -F "file=@local.txt" \
  http://localhost:30003/api/upload
```
