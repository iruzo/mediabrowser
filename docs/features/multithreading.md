# Multithreading

The optional `multithreading` feature enables Tokio's `rt-multi-thread` scheduler
and runs asynchronous tasks across worker threads. It is disabled by default
and works independently of the other features.

```sh
cargo run --locked --features multithreading
```

For containers:

```sh
FEATURES=api,ui,multithreading docker compose --profile pro up --build
```

Without this feature, asynchronous tasks run on the current thread. Blocking
operations dispatched through `spawn_blocking` use separate threads in both
configurations. Authentication's password hashing remains serialized.

Tokio chooses the worker count from the available CPU cores by default.
Multithreading can improve throughput under concurrent load, with additional
scheduling overhead, memory use, and potentially a larger binary. Actual gains
depend on the workload; disk and network limits still apply.

`--all-features` also enables multithreading.
