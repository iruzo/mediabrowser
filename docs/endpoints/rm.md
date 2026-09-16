# POST /api/rm

Remove a file or directory using a URL-encoded `path` field.
Directories are removed recursively.

```html
<form method="post" action="/api/rm">
  <label>Path <input name="path" value="folder/file.txt" required></label>
  <button type="submit">Remove</button>
</form>
```

```sh
curl -X POST \
  -d "path=folder/file.txt" \
  http://localhost:30003/api/rm
```
