# Binary size

The release profile uses size optimization (`opt-level = "z"`), full link-time
optimization, one code generation unit, aborting panics, and symbol stripping.

Filesystem calls in copy, move, and find pass borrowed `&Path` values to Tokio.
Using the same argument type as other callers avoids generating separate async
implementations for `PathBuf`, `&PathBuf`, and `&Path`. The paths, filesystem
operations, validation, and error responses are unchanged.

TAR downloads poll the existing bounded channel directly. This avoids an extra
async state machine that moved the receiver between stream iterations. Channel
capacity, chunk sizes, backpressure, and error propagation are unchanged.

## Measurements

Measured with Rust 1.94.1 on `x86_64-unknown-linux-musl`, using the locked
dependencies and existing release profile. UI assets were minified as in the
Dockerfile. Each comparison uses the same features and build settings.
Sizes are executable file sizes in bytes, not container image sizes.

| Features | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| API + UI (default) | 906,016 | 897,824 | 8,192 |
| API | 889,632 | 885,536 | 4,096 |
| HTTPD + API | 914,208 | 906,016 | 8,192 |
| HTTPD + API + UI | 926,496 | 922,400 | 4,096 |
| HTTPD | 750,336 | 750,336 | 0 |

Sizes can vary with the compiler, target, dependencies, and asset minification.
All existing tests passed for these five combinations. Live comparisons against
the original binaries covered all eight API endpoints using URL-encoded and
multipart forms, file ranges, UI gzip negotiation, HTTPD, TAR contents, canceled
downloads, and graceful shutdown during an active download.
