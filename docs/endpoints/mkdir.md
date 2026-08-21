# POST /api/mkdir

Recursively create a directory using a URL-encoded `path` field.

```sh
curl -X POST \
  -d "path=folder/subfolder" \
  http://localhost:30003/api/mkdir
```
