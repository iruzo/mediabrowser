# POST /api/upload

Upload files using multipart `path` and `file` fields (256GB limit).
The `path` field must precede the `file` fields. Colliding names
get a numeric suffix before the extension.
Quoted multipart filenames preserve semicolons and escaped quotes.

```html
<form method="post" action="/api/upload" enctype="multipart/form-data">
  <label>Destination <input name="path" value="folder"></label>
  <label>Files <input type="file" name="file" multiple required></label>
  <button type="submit">Upload</button>
</form>
```

```sh
curl -X POST \
  -F "path=folder" \
  -F "file=@local.txt" \
  http://localhost:30003/api/upload
```
