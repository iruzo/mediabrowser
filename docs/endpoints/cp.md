# POST /api/cp

Copy one file or directory using URL-encoded `from` and `to` fields.
Directories are copied recursively without following symlinks.
Fails with 409 if the destination exists.

```sh
curl -X POST \
  -d "from=folder/file.txt" \
  -d "to=folder/copy.txt" \
  http://localhost:30003/api/cp
```
