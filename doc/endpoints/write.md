# POST /api/write

Create or replace a UTF-8 text file using URL-encoded
`path` and `content` fields (16MB limit).

```sh
curl -X POST \
  --data-urlencode "path=folder/notes.txt" \
  --data-urlencode "content=hello" \
  http://localhost:30003/api/write
```
