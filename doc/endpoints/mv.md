# POST /api/mv

Move one path using URL-encoded `from` and `to` fields.
Fails with 409 if the destination exists.

```sh
curl -X POST \
  -d "from=folder/file.txt" \
  -d "to=folder/renamed.txt" \
  http://localhost:30003/api/mv
```
