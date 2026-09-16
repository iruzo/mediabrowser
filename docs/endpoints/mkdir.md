# POST /api/mkdir

Recursively create a directory using a URL-encoded `path` field.

```html
<form method="post" action="/api/mkdir">
  <label>Directory <input name="path" value="folder/subfolder" required></label>
  <button type="submit">Create directory</button>
</form>
```

```sh
curl -X POST \
  -d "path=folder/subfolder" \
  http://localhost:30003/api/mkdir
```
