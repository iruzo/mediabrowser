# POST /api/cat

Return one regular file selected by a URL-encoded `path` form field. The
response uses the file's media type and supports byte ranges.

Missing paths, directories, symbolic links, and invalid paths return `404 Not
Found`.

```html
<form method="post" action="/api/cat">
  <input type="hidden" name="path" value="photos/image.jpg">
  <button type="submit">Open</button>
</form>
```

```sh
curl \
  --data-urlencode "path=folder/file.txt" \
  http://localhost:30003/api/cat
```

The POST response can be displayed as a document or in a form target. HTML
media elements cannot submit POST requests directly.
When built with `httpd`, use [direct file URLs](httpd.md) for HTML media
elements instead.
