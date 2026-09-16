# POST /api/find

List or search paths as JSON. By default the response is an array of strings.
Directories end with `/`. Parameters are submitted as URL-encoded form fields,
and all are optional:
`path` scopes the walk, `type` keeps `dir`, `file`, or `all` paths before
query filtering, and `query` filters the returned paths by space-separated
terms (max 500 results when searching). `type` defaults to `all`.
An empty `type` also leaves both files and directories in the results.
`recursive=false` lists only immediate children; it defaults to `true`.
`metadata=true` returns objects with `path`, byte `size`, Unix-second `date`, and
`kind` fields instead of strings; `kind` is `image`, `video`, `audio`, or `text`.
Directories and formats outside image, video, or audio use `text`; it is a UI
fallback rather than a MIME guarantee. Metadata defaults to `false`.
Matching is case-insensitive unless the query contains an
uppercase letter.

The HTML form below submits the search, but the response remains JSON.
For directory browsing without JavaScript, use the optional
[HTTPD file server](httpd.md).

```html
<form method="post" action="/api/find">
  <input name="path" value="folder">
  <input name="query" value="report">
  <input type="hidden" name="type" value="dir">
  <button type="submit">Find</button>
</form>
```

```sh
curl \
  --data-urlencode "path=folder" \
  --data-urlencode "query=report" \
  --data-urlencode "type=dir" \
  --data-urlencode "recursive=true" \
  http://localhost:30003/api/find
```
