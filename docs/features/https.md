# HTTPS

The `https` feature enables HTTPS with an in-memory self-signed certificate.
The certificate and private key are regenerated on every startup.

```sh
cargo build --locked --release --features https
cargo run --locked --features https
```

Set `TLS_HOSTS` to the DNS names and IP addresses clients use:

```sh
BIND_ADDR=0.0.0.0 TLS_HOSTS=media.local,192.168.1.10 cargo run --locked --features https
```

`TLS_HOSTS` is comma-separated. The default certificate covers `localhost`,
`127.0.0.1`, and `::1`. Browsers may require the self-signed certificate to be
accepted again after each restart. For local testing:

```sh
curl --insecure --compressed https://localhost:30003/ui/
```

The listener accepts IPv4 and IPv6 `BIND_ADDR` values. Use `::1` for IPv6
