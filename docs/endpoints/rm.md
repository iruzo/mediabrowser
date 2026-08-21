# POST /api/rm

Remove a file or directory using a URL-encoded `path` field.
Directories are removed recursively.

```sh
curl -X POST \
  -d "path=folder/file.txt" \
  http://localhost:30003/api/rm
```
