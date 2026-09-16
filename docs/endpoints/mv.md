# POST /api/mv

Move one path using URL-encoded `from` and `to` fields.
Fails with 409 if the destination exists.

```html
<form method="post" action="/api/mv">
  <label>Source <input name="from" value="folder/file.txt" required></label>
  <label>Destination <input name="to" value="folder/renamed.txt" required></label>
  <button type="submit">Move</button>
</form>
```

```sh
curl -X POST \
  -d "from=folder/file.txt" \
  -d "to=folder/renamed.txt" \
  http://localhost:30003/api/mv
```
