<h3 align="center">
    A simple, lightweight file server
</h3>

<div align="center">
	<img src="./assets/preview.webp"/>
</div>

## Compile

Build the release binary at `target/release/mediabrowser`:

```sh
cargo build --locked --release
```

The default build enables `api` and `ui`. To enable all features:

```sh
cargo build --locked --release --all-features
```

Or build the container image:

```sh
docker compose --profile pro build
```

## Documentation

See the [manual](./docs/manual.md) for running, configuration, features, and endpoints.

## TODO

- Text file editing (?)
