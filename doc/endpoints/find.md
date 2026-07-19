# GET /api/find

Recursively list or search paths as JSON strings.
Directories end with `/`. Both parameters are optional:
`path` scopes the walk, `query` filters the returned paths
by space-separated terms (max 500 results when searching).
Matching is case-insensitive unless the query contains an
uppercase letter.

```sh
curl "http://localhost:30003/api/find?path=folder&query=report"
```

```sh
curl -G \
  --data-urlencode "path=folder" \
  --data-urlencode "query=report" \
  http://localhost:30003/api/find
```
