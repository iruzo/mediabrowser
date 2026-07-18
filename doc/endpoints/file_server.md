# GET /

Apache-style httpd at the root path. Directories return an HTML
listing, files are served inline with range support.

```sh
curl "http://localhost:30003/folder/"
curl -O "http://localhost:30003/folder/file.mp4"
```
