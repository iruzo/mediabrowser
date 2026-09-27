# UI

The `ui` feature enables the media browser under `/ui/`. It also enables the
`api` feature because files are loaded through the API.

```sh
cargo run --locked --features ui
```

Open `http://localhost:30003/ui/` in a browser. Directory paths under `/ui/`
refer to the same data directory exposed by the HTTPD feature.
