# POST /api/downloads

Stream selected paths as TAR using repeated URL-encoded
`path` fields (max 1024 paths).

```sh
curl -X POST \
  -d "path=folder" \
  -d "path=file.txt" \
  -o download.tar \
  http://localhost:30003/api/downloads
```
