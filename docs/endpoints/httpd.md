# HTTPD file server

The optional `httpd` feature serves files at the root path, separately from
the POST API. GET requests select a path through the URL. Directories return
an HTML listing; files are served inline with range support.

Directory URLs without a trailing slash redirect with `308 Permanent Redirect`
to the trailing-slash URL, so relative links resolve inside the directory.

HTML pages can link to directories and embed files without JavaScript:

```html
<a href="/folder/">Browse folder</a>
<a href="/folder/file.txt">Open file</a>
<video controls src="/folder/file.mp4"></video>
```

```sh
curl "http://localhost:30003/folder/"
curl -O "http://localhost:30003/folder/file.mp4"
```
