# GET /api/find

List or search paths as JSON. By default the response is an array of strings.
Directories end with `/`. All parameters are optional:
`path` scopes the walk, `type` keeps `dir`, `file`, or `all` paths before
query filtering, and `query` filters the returned paths by space-separated
terms (max 500 results when searching). `type` defaults to `all`.
An empty `type` also leaves both files and directories in the results.
`recursive=false` lists only immediate children; it defaults to `true`.
`metadata=true` returns objects with `path`, byte `size`, and Unix-second
`date` fields instead of strings; it defaults to `false`.
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
  --data-urlencode "recursive=true" \
  http://localhost:30003/api/find
```
