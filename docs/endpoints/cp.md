# POST /api/cp

Copy one file or directory using URL-encoded `from` and `to` fields.
Directories are copied recursively without following symlinks.
Fails with 409 if the destination exists.

```html
<form method="post" action="/api/cp">
  <label>Source <input name="from" value="folder/file.txt" required></label>
  <label>Destination <input name="to" value="folder/copy.txt" required></label>
  <button type="submit">Copy</button>
</form>
```

```sh
curl -X POST \
  -d "from=folder/file.txt" \
  -d "to=folder/copy.txt" \
  http://localhost:30003/api/cp
```
