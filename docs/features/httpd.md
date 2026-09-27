# HTTPD

The `httpd` feature serves an Apache-like directory listing and regular files
at the root path.

```sh
cargo run --locked --no-default-features --features httpd
```

Examples:

- `http://localhost:30003/` lists the root directory.
- `http://localhost:30003/folder/` lists a folder.
- `http://localhost:30003/folder/file.mp4` serves a file.

See the [HTTPD endpoint notes](../endpoints/httpd.md) for behavior details.
