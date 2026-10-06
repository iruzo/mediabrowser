# UI

The `ui` feature enables the media browser under `/ui/`. It also enables the
`api` feature because files are loaded through the API.

```sh
cargo run --locked --features ui
```

Open `http://localhost:30003/ui/` in a browser. Directory paths under `/ui/`
refer to the same data directory exposed by the HTTPD feature.

UI assets live in `static/ui/`. JavaScript is split by responsibility:

- `common.js`: shared helpers and API calls.
- `state.js`: DOM references and shared state.
- `gallery.js`: directory loading, thumbnails, and pagination.
- `actions.js`: selection, file operations, search, sorting, and uploads.
- `viewer.js`: media viewing, playback controls, and navigation.
- `init.js`: initial directory loading.

`build.rs` assembles these files in an explicit order into the existing embedded,
gzipped page. They share the same script scope; no JavaScript bundler or extra
asset requests are required. It selects `request-auth.js` only with the `auth`
feature; other builds use `request.js`. When adding a script, update the ordered
list in `build.rs`. The standalone forms are `static/login.html` and `static/password.html` and
need no JavaScript. Docker copies and minifies the entire `static/` directory.
