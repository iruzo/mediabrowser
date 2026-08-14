# GET /api/find

Recursively list or search paths as JSON strings.
Directories end with `/`. All parameters are optional:
`path` scopes the walk, `type` keeps `dir`, `file`, or `all` paths before
query filtering, and `query` filters the returned paths by space-separated
terms (max 500 results when searching). `type` defaults to `all`.
An empty `type` also leaves both files and directories in the results.
Matching is case-insensitive unless the query contains an
uppercase letter.

```sh
curl "http://localhost:30003/api/find?path=folder&query=report&type=dir"
```

```sh
curl -G \
  --data-urlencode "path=folder" \
  --data-urlencode "query=report" \
  --data-urlencode "type=dir" \
  http://localhost:30003/api/find
```
